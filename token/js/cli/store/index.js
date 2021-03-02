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

  static getFilename(uri: string): string {
    return path.join(Store.getDir(), uri);
  }

  async load(uri: string): Promise<Object> {
    const filename = Store.getFilename(uri);
    const data = await fs.readFile(filename, 'utf8');
    const config = JSON.parse(data);
    return config;
  }

  async save(uri: string, config: Object): Promise<void> {
    await mkdirp(Store.getDir());
    const filename = Store.getFilename(uri);
    await fs.writeFile(filename, JSON.stringify(config), 'utf8');
  }
}
