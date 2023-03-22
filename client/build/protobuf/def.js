import { safeProp, toJsType } from './helpers.js';

export default type => {
  const code = [];
  const push = line => code.push(line);

  push(`export const def${type.name} = (): ${type.name} => {`);

  push(`  return {`);
  type.fieldsArray.forEach(field => {
    const type = toJsType(field);
    let value;

    if(type == 'number') value = '0';
    else if(type == 'boolean') value = 'false';
    else if(type == 'string') value = '\'\'';
    else if(type == 'Uint8Array') value = 'new Uint8Array()';
    else if(type.startsWith('Record')) value = '{}';
    else if(type.endsWith('[]')) value = '[]';
    else value = `def${type}();`;

    push(`    ${safeProp(field.name)}: ${value},`);
  });

  code[code.length - 1] = code[code.length - 1].slice(0, -1);

  push('  };');
  push('};');

  return code.join('\n');
}