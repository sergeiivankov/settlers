import protobufjs from 'protobufjs';
import decoder from './decoder.js';
import def from './def.js';
import encoder from './encoder.js';
import { safeProp, toJsType } from './helpers.js';

let imports = new Set();
let code = [];

const push = line => code.push(line);

const generateType = type => {
  push('');

  push(`export interface ${type.name} {`)
  type.fieldsArray.forEach(field => {
    push(`  ${safeProp(field.name)}?: ${toJsType(field)}|null|undefined;`);
  });
  push('}');

  push('');
  push(def(type));

  const encoderData = encoder(type);
  const decoderData = decoder(type);

  imports = new Set([...imports, ...encoderData.imports, ...decoderData.imports])

  push('');
  push(encoderData.code);
  push('');
  push(decoderData.code);
};

const generateNamespace = ns => {
  if(!ns || ns instanceof protobufjs.Service) return;

  if(ns instanceof protobufjs.Type) generateType(ns);

  ns.nestedArray.forEach(nested => {
    if(nested instanceof protobufjs.Namespace) generateNamespace(nested);
  });
};

const generate = root => {
  generateNamespace(root);
  code.unshift(`import { ${Array.from(imports).sort().join(', ')} } from '../helpers/protobuf';`);
  return code.join("\n");
};

export default generate;