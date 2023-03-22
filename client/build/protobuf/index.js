import { readdirSync, mkdirSync, readFileSync, statSync, writeFileSync } from 'fs';
import protobufjs from 'protobufjs';
import generate from './generator.js';

const walk = dir => {
  let results = [];

  const list = readdirSync(dir);
  list.forEach(file => {
    file = dir + '/' + file;
    const stat = statSync(file);
    if(stat && stat.isDirectory()) results = results.concat(walk(file));
    else results.push(file);
  });

  return results;
}

export default (inDir, outDir) => {
  walk(inDir).forEach(file => {
    const code = readFileSync(file, 'utf8');
    const root = new protobufjs.Root();

    protobufjs.parse(code, root, {
      alternateCommentMode: false,
      keepCase: false
    });

    const generated = generate(root);

    const outFile = `${outDir}/${file.slice(inDir.length).split('.').slice(0, -1).join('.')}.ts`;
    mkdirSync(outFile.split('/').slice(0, -1).join('/'), { recursive: true })

    writeFileSync(outFile, generated, 'utf8');
  });
};