import '../scss/index.scss';

import { encodeCheckTokenParams, decodeCheckTokenResult } from './protos/auth';

fetch('/api/check_token', {
  method: 'POST',
  body: encodeCheckTokenParams({ token: '12345678' })
}).then(res => res.arrayBuffer()).then(buffer => {
  const data = new Uint8Array(buffer);
  console.log(data, decodeCheckTokenResult(data, 0))
})