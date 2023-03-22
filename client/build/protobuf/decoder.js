import { capitalizeFirstLetter, safeProp } from './helpers.js';

export default type => {
  const imports = new Set(['readUnknown']);

  const code = [];
  const push = line => code.push(line);

  const fnName = `decode${type.name}`;
  push(`export const ${fnName} = (buffer: Uint8Array, offset: number): ${type.name}|null => {`);
  push(`  const data = def${type.name}();`);

  push('');
  push('  let position = offset;');
  push('  const length = buffer.length;');

  push('');
  push('  while(position < length) {');

  imports.add('readUint32');
  push('    const [readed, tag] = readUint32(buffer, position);');
  push('    if(readed) { position += readed } else { return null; }');

  push('');
  push('    switch(tag >>> 3) {');

  for(let i = 0; i < type.fieldsArray.length; i++) {
    const field = type._fieldsArray[i].resolve();

    const safeName = safeProp(field.name);
    const ref = safeName === field.name ? `data.${safeName}` : `data[${safeName}]`;

    const typeReader = `read${capitalizeFirstLetter(field.type)}`;
    imports.add(typeReader);

    push(`      case ${field.id}: {`);
    push(`        const [readed, value] = ${typeReader}(buffer, position);`);
    push('        if(readed) { position += readed } else { return null; }');
    push(`        ${ref} = value;`);
    push('        break;');
    push('      }');
  }

  push('      default: {');
  push('        const [readed] = readUnknown(buffer, position, tag & 7);');
  push('        if(readed) { position += readed } else { return null; }');
  push('        break;');
  push('      }');

  push(`    }`);
  push(`  }`);

  push('');
  push('  return data;');
  push('};');

  return { code: code.join('\n'), imports };
}