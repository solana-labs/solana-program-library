/**
 * Simple file-based datastore
 */

import path from 'path';
import fs from 'mz/fs';
import mkdirp from 'mkdirp';

export class Store {
  static getDir(): string {
    return path.join(__dirname, 'store');
  }

  async load(uri: string): Promise<Record<string, string>> {
    const filename = path.join(Store.getDir(), uri);
    const data = await fs.readFile(filename, 'utf8');
    return JSON.parse(data);
  }

  async save(uri: string, config: Record<string, string>): Promise<void> {
    await mkdirp(Store.getDir());
    const filename = path.join(Store.getDir(), uri);
    await fs.writeFile(filename, JSON.stringify(config), 'utf8');
  }
}
