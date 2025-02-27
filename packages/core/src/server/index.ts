import { readFileSync } from 'node:fs';
import http from 'node:http';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import Koa from 'koa';
import { WebSocketServer } from 'ws';
import chalk from 'chalk';
import boxen from 'boxen';
import figlet from 'figlet';
import { Compiler } from '../compiler/index.js';
import {
  UserServerConfig,
  NormalizedServerConfig,
  normalizeDevServerOptions,
  normalizePublicDir,
  normalizePublicPath,
  DevServerPlugin,
  UserConfig,
  DEFAULT_HMR_OPTIONS,
  urlRegex
} from '../config/index.js';
import { HmrEngine } from './hmr-engine.js';
import { openBrowser } from './openBrowser.js';
import { brandColor, clearScreen, Logger } from '../utils/index.js';
import {
  lazyCompilationPlugin,
  resourcesPlugin,
  hmrPlugin,
  proxyPlugin,
  corsPlugin,
  recordsPlugin,
  headersPlugin
} from './middlewares/index.js';
import { __FARM_GLOBAL__ } from '../config/_global.js';

/**
 * Farm Dev Server, responsible for:
 * * parse and normalize dev server options
 * * launch http server based on options
 * * compile the project in dev mode and serve the production
 * * HMR middleware and websocket supported
 */

interface FarmServerContext {
  config: UserServerConfig;
  app: Koa;
  server: http.Server;
  compiler: Compiler;
  logger: Logger;
}

interface ImplDevServer {
  createFarmServer(options: UserServerConfig): void;
  listen(): Promise<void>;
  close(): Promise<void>;
  getCompiler(): Compiler;
}

export class DevServer implements ImplDevServer {
  private _app: Koa;
  public _context: FarmServerContext;

  ws: WebSocketServer;
  config: NormalizedServerConfig;
  hmrEngine?: HmrEngine;
  server?: http.Server;
  publicDir?: string;
  publicPath?: string;
  userConfig?: UserConfig;

  constructor(
    private _compiler: Compiler,
    public logger: Logger,
    options?: UserConfig
  ) {
    this.publicDir = normalizePublicDir(
      _compiler.config.config.root,
      options.publicDir
    );

    this.publicPath =
      normalizePublicPath(
        options?.compilation?.output?.publicPath,
        logger,
        false
      ) || '/';

    this.userConfig = options;
    this.createFarmServer(options.server);
  }

  getCompiler(): Compiler {
    return this._compiler;
  }

  app(): Koa {
    return this._app;
  }

  async listen(): Promise<void> {
    if (!this.server) {
      this.logger.error('HTTP server is not created yet');
    }
    const { port, open, protocol, hostname, host } = this.config;
    const start = Date.now();
    let publicPath;
    if (urlRegex.test(this.publicPath)) {
      publicPath = '/';
    } else {
      publicPath = this.publicPath.startsWith('/')
        ? this.publicPath
        : `/${this.publicPath}`;
    }
    // compile the project and start the dev server
    if (process.env.FARM_PROFILE) {
      this._compiler.compileSync();
    } else {
      await this._compiler.compile();
    }

    if (this.config.writeToDisk) {
      const base = this.publicPath.match(/^https?:\/\//) ? '' : this.publicPath;
      this._compiler.writeResourcesToDisk(base);
    }

    const end = Date.now();
    this.server.listen(port, host);
    this.error(port, host);
    __FARM_GLOBAL__.__FARM_RESTART_DEV_SERVER__ &&
      this.startDevLogger(start, end);
    if (open) {
      openBrowser(`${protocol}://${hostname}:${port}${publicPath}`);
    }
  }

  async error(port: number, ip: string | boolean) {
    // TODO error
    // TODO Callback handling of all errors extracted from the function
    function handleError(
      error: Error & { code?: string },
      port: number,
      ip: string | boolean
    ) {
      // TODO ip boolean type true ... false
      const errorMap: any = {
        EADDRINUSE: `Port ${port} is already in use`,
        EACCES: `Permission denied to use port ${port}`,
        EADDRNOTAVAIL: `The IP address ${ip} is not available on this machine.`
      };

      const errorMessage =
        errorMap[error.code] || `An error occurred: ${error}`;
      this.logger.error(errorMessage);
    }
    this.server.on('error', (error: Error & { code?: string }) => {
      handleError(error, port, ip);
      this.server.close(() => {
        process.exit(1);
      });
    });
  }

  async close() {
    if (!this.server) {
      this.logger.error('HTTP server is not created yet');
    }
    await this.closeFarmServer();
  }

  async restart() {
    // TODO restart
  }

  async closeFarmServer() {
    await this.server.close();
  }

  createFarmServer(options: UserServerConfig) {
    const { https = false, host = 'localhost', plugins = [] } = options;
    const protocol = https ? 'https' : 'http';
    const hostname = host === '0.0.0.0' ? 'localhost' : host;
    this.config = normalizeDevServerOptions(
      { ...options, protocol, hostname },
      'development'
    );

    this._app = new Koa();
    this.server = http.createServer(this._app.callback());
    this._context = {
      config: this.config,
      app: this._app,
      server: this.server,
      compiler: this._compiler,
      logger: this.logger
    };
    this.resolvedFarmServerPlugins(plugins);
  }

  static async resolvePortConflict(
    userConfig: UserConfig,
    logger: Logger
  ): Promise<void> {
    const normalizedDevConfig = normalizeDevServerOptions(
      userConfig.server,
      'development'
    );
    userConfig.server = normalizedDevConfig;

    let devPort = normalizedDevConfig.port;
    let hmrPort = DEFAULT_HMR_OPTIONS.port;
    const { strictPort, host } = normalizedDevConfig;
    const httpServer = http.createServer();
    const isPortAvailable = (portToCheck: number) => {
      return new Promise((resolve, reject) => {
        const onError = async (error: { code: string }) => {
          if (error.code === 'EADDRINUSE') {
            clearScreen();
            if (strictPort) {
              httpServer.removeListener('error', onError);
              reject(new Error(`Port ${devPort} is already in use`));
            } else {
              logger.warn(`Port ${devPort} is in use, trying another one...`);
              httpServer.removeListener('error', onError);
              resolve(false);
            }
          } else {
            reject(true);
          }
        };
        httpServer.on('error', onError);
        httpServer.on('listening', () => {
          httpServer.close();
          resolve(true);
        });
        httpServer.listen(portToCheck, host);
      });
    };

    let isPortAvailableResult = await isPortAvailable(devPort);
    while (isPortAvailableResult === false) {
      userConfig.server.hmr = { port: ++hmrPort };
      userConfig.server.port = ++devPort;
      isPortAvailableResult = await isPortAvailable(devPort);
    }
  }

  /**
   *
   * Add listening files for root manually
   *
   * > listening file with root must as file.
   *
   * @param root
   * @param deps
   */
  addWatchFile(root: string, deps: string[]) {
    this.getCompiler().addExtraWatchFile(root, deps);
  }

  private resolvedFarmServerPlugins(middlewares?: DevServerPlugin[]) {
    const resolvedPlugins = [
      ...middlewares,
      headersPlugin,
      lazyCompilationPlugin,
      hmrPlugin,
      corsPlugin,
      resourcesPlugin,
      recordsPlugin,
      proxyPlugin
    ];
    // this._app.use(serve(this._dist));
    resolvedPlugins.forEach((plugin) => plugin(this));
  }

  private startDevLogger(start: number, end: number) {
    const { port, protocol, hostname } = this.config;
    const version = JSON.parse(
      readFileSync(
        join(fileURLToPath(import.meta.url), '../../../package.json'),
        'utf-8'
      )
    ).version;
    let publicPath;
    if (urlRegex.test(this.publicPath)) {
      publicPath = '/';
    } else {
      publicPath = this.publicPath.startsWith('/')
        ? this.publicPath
        : `/${this.publicPath}`;
    }
    this.logger.info(
      boxen(
        `${brandColor(
          figlet.textSync('FARM', {
            width: 40
          })
        )}
  Version ${chalk.green.bold(version)}

  🔥 Ready on ${chalk.green.bold(
    `${protocol}://${hostname}:${port}${publicPath}`
  )} in ${chalk.green.bold(`${end - start}ms`)}.
    `,
        {
          padding: 1,
          margin: 1,
          align: 'center',
          borderColor: 'cyan',
          borderStyle: 'round'
        }
      ),
      false
    );
  }
}
