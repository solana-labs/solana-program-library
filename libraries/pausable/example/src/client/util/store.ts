/**
 * Simple file-based datastore
 */

import path from 'path';
import fs from 'mz/fs';
import mkdirp from 'mkdirp';

type Config = {[key: string]: string};

export class Store {
  static getDir(): string {
    return path.join(__dirname, 'store');
  }

  async load(uri: string): Promise<Config> {
    const filename = path.join(Store.getDir(), uri);
    const data = await fs.readFile(filename, 'utf8');
    return JSON.parse(data) as Config;
  }

  async save(uri: string, config: Config): Promise<void> {
    await mkdirp(Store.getDir());
    const filename = path.join(Store.getDir(), uri);
    await fs.writeFile(filename, JSON.stringify(config), 'utf8');
  }
}
