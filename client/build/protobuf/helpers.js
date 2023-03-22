const noSpecCharsRegExp = /^[$\w_]+$/;

const reservedRegExp = /^(?:do|if|in|for|let|new|try|var|case|else|enum|eval|false|null|this|true|void|with|break|catch|class|const|super|throw|while|yield|delete|export|import|public|return|static|switch|typeof|default|extends|finally|package|private|continue|debugger|function|arguments|interface|protected|implements|instanceof)$/;

const backslashRegExp = /\\/g;
const quoteRegExp = /'/g;

export const types = {
  double: 1,
  float: 5,
  int32: 0,
  uint32: 0,
  sint32: 0,
  fixed32: 5,
  sfixed32: 5,
  int64: 0,
  uint64: 0,
  sint64: 0,
  fixed64: 1,
  sfixed64: 1,
  bool: 0,
  string: 2,
  bytes: 2
};

export const capitalizeFirstLetter = string => {
  return string.charAt(0).toUpperCase() + string.slice(1);
};

export const safeProp = prop => {
  if(noSpecCharsRegExp.test(prop) && !reservedRegExp.test(prop)) return prop;
  return `'${prop.replace(backslashRegExp, '\\\\').replace(quoteRegExp, "\\\'")}'`;
};

export const toJsType = field => {
  let type;

  switch(field.type) {
    case 'double':
    case 'float':
    case 'int32':
    case 'uint32':
    case 'sint32':
    case 'fixed32':
    case 'sfixed32':
    case 'int64':
    case 'uint64':
    case 'sint64':
    case 'fixed64':
    case 'sfixed64':
      type = 'number';
      break;
    case 'bool':
      type = 'boolean';
      break;
    case 'string':
      type = 'string';
      break;
    case 'bytes':
      type = 'Uint8Array';
      break;
    default:
      if(field.resolve().resolvedType) type = object.name;
      else throw new Error(`Can\'t convert type: ${field.name}`);
      break;
  }
  if(field.map) return `Record<string, ${type}>`;
  if(field.repeated) return type + '[]';

  return type;
};