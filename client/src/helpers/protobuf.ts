const utf8Length = (string: string): number => {
  let length = 0;

  for(let i = 0; i < string.length; i++) {
    const char = string.charCodeAt(i);

    if(char < 128) length += 1;
    else if (char < 2048) length += 2;
    else if ((char & 0xFC00) === 0xD800 && (string.charCodeAt(i + 1) & 0xFC00) === 0xDC00) {
      i++;
      length += 4;
    }
    else length += 3;
  }

  return length;
};

const utf8Read = (buffer: number[]): string => {
  let string = '';

  for(var i = 0; i < buffer.length;) {
    const c1 = buffer[i++];

    if(c1 <= 0x7F) {
      string += String.fromCharCode(c1);
    } else if (c1 >= 0xC0 && c1 < 0xE0) {
      string += String.fromCharCode((c1 & 0x1F) << 6 | buffer[i++] & 0x3F);
    } else if (c1 >= 0xE0 && c1 < 0xF0) {
      string += String.fromCharCode(
        (c1 & 0xF) << 12 | (buffer[i++] & 0x3F) << 6 | buffer[i++] & 0x3F
      );
    } else if (c1 >= 0xF0) {
      const c2 = (
        (c1 & 7) << 18 | (buffer[i++] & 0x3F) << 12 | (buffer[i++] & 0x3F) << 6 | buffer[i++] & 0x3F
      ) - 0x10000;

      string += String.fromCharCode(0xD800 + (c2 >> 10));
      string += String.fromCharCode(0xDC00 + (c2 & 0x3FF));
    }
  }

  return string;
};

const utf8Write = (buffer: number[], string: string): void => {
  let c1: number;
  let c2: number;

  for(let i = 0; i < string.length; i++) {
    c1 = string.charCodeAt(i);

    if(c1 < 128) buffer.push(c1);
    else if(c1 < 2048) {
      buffer.push(c1 >> 6 | 192);
      buffer.push(c1 & 63 | 128);
    } else if((c1 & 0xFC00) === 0xD800 && ((c2 = string.charCodeAt(i + 1)) & 0xFC00) === 0xDC00) {
      c1 = 0x10000 + ((c1 & 0x03FF) << 10) + (c2 & 0x03FF);
      i++;

      buffer.push(c1 >> 18 | 240);
      buffer.push(c1 >> 12 & 63 | 128);
      buffer.push(c1 >> 6 & 63 | 128);
      buffer.push(c1 & 63 | 128);
    } else {
      buffer.push(c1 >> 12 | 224);
      buffer.push(c1 >> 6 & 63 | 128);
      buffer.push(c1 & 63 | 128);
    }
  }
};

export const readBool = (buffer: Uint8Array, position: number): [number, boolean] => {
  const [readed, number] = readUint32(buffer, position);
  return [readed, number !== 0];
};

export const readBytes = (buffer: Uint8Array, position: number): [number, number[]] => {
  const [readed, length] = readUint32(buffer, position);

  const start = position + readed;
  const end = start + length;

  if (end > buffer.length) return [0, []];

  return [readed + length, Array.from(buffer.slice(start, end))];
};

export const readString = (buffer: Uint8Array, position: number): [number, string] => {
  const [readed, bytes] = readBytes(buffer, position);
  return [readed, utf8Read(bytes)];
};

export const readUint32 = (buffer: Uint8Array, position: number): [number, number] => {
  let number: number;

  number = (buffer[position] & 127) >>> 0;
  if(buffer[position++] < 128) return [1, number];

  number = (number | (buffer[position] & 127) << 7) >>> 0;
  if(buffer[position++] < 128) return [2, number];

  number = (number | (buffer[position] & 127) << 14) >>> 0;
  if(buffer[position++] < 128) return [3, number];

  number = (number | (buffer[position] & 127) << 21) >>> 0;
  if(buffer[position++] < 128) return [4, number];

  number = (number | (buffer[position] & 15) << 28) >>> 0;
  if(buffer[position++] < 128) return [5, number];

  if(position + 5 > buffer.length) return [0, 0];

  return [5, number];
};

const skip = (buffer: Uint8Array, position: number, length: number): number => {
  if(length > 0) {
    if(position + length > buffer.length) return 0;
    return length;
  }

  let skipped = 0;
  do {
    if(position > buffer.length) return 0;
    skipped++;
  } while (buffer[position++] & 128);

  return skipped;
};

export const readUnknown = (buffer: Uint8Array, position: number, type: number): [number] => {
  if(type === 0) return [skip(buffer, position, 0)];
  if(type === 1) return [skip(buffer, position, 8)];
  if(type === 5) return [skip(buffer, position, 4)];

  if(type === 2) {
    const [readed, size] = readUint32(buffer, position);
    if(readed) { position += readed } else { return [0]; }
    return [readed + skip(buffer, position, size)];
  }

  if(type === 3) {
    while(true) {
      let [readed, tag] = readUint32(buffer, position);
      if(readed) { position += readed } else { return [0]; }

      tag = tag & 7;
      if(tag === 4) break;

      readUnknown(buffer, position, type);
    }
  }

  return [0];
};

export const writeBool = (buffer: number[], boolean: boolean): void => {
  buffer.push(boolean ? 1 : 0);
};

export const writeString = (buffer: number[], string: string): void => {
  const length = utf8Length(string);

  if(length === 0) buffer.push(0);
  else {
    writeUint32(buffer, length);
    utf8Write(buffer, string);
  }
};

export const writeUint32 = (buffer: number[], number: number): void => {
  while(number > 127) {
    buffer.push(number & 127 | 128);
    number >>>= 7;
  }
  buffer.push(number);
};