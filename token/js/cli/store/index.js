/**
 * Simple file-based datastore
 *
 * @flow
 */

import path from 'path';
import fs from 'mz/fs';
import mkdirp from 'mkdirp';

export class Store {
  static getDir(): string {
    return path.join(__dirname, 'store');
  }

  async load(uri: string): Promise<Object> {
    const filename = path.join(Store.getDir(), uri);
    const data = await fs.readFile(filename, 'utf8');
    const config = JSON.parse(data);
    return config;
  }

  async save(uri: string, config: Object): Promise<void> {
    await mkdirp(Store.getDir());
    const filename = path.join(Store.getDir(), uri);
    await fs.writeFile(filename, JSON.stringify(config), 'utf8');
  }
}
