import { capitalizeFirstLetter, safeProp, types } from './helpers.js';

export default type => {
  const imports = new Set();

  const code = [];
  const push = line => code.push(line);

  push(`export const encode${type.name} = (data: ${type.name}): Uint8Array => {`);
  push(`  const result = [];`);

  const fields = type.fieldsArray.slice().sort((a, b) => a.id - b.id);

  for(var i = 0; i < fields.length; ++i) {
    const field = fields[i].resolve();
    const basicType = types[field.type];

    const safeName = safeProp(field.name);
    const ref = safeName === field.name ? `data.${safeName}` : `data[${safeName}]`;

    push('');

    const property = safeName.startsWith('\'') ? safeName : `'${safeName}'`;
    push(`  if(${ref} != null && data.hasOwnProperty(${property})) {`);

    const typeWriter = `write${capitalizeFirstLetter(field.type)}`;
    imports.add(typeWriter);

    push(`    writeUint32(result, ${(field.id << 3 | basicType) >>> 0});`);
    push(`    ${typeWriter}(result, ${ref});`);
    push(`  }`);

    imports.add('writeUint32');
  }

  push('');
  push('  return Uint8Array.from(result);');
  push('};');

  return { code: code.join('\n'), imports };
}