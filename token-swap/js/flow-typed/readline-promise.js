declare module 'readline-promise' {

  declare class ReadLine {
    questionAsync(prompt: string): Promise<string>;
    write(text: string): void;
  }

  declare module.exports: {
    createInterface({
      input: Object,
      output: Object,
      terminal: boolean
    }): ReadLine;
  }
}
