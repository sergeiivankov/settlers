import { readFileSync } from 'fs';
import copy from 'rollup-plugin-copy';
import sass from 'rollup-plugin-sass';
import typescript from '@rollup/plugin-typescript';
import buildProtobuf from './build/protobuf/index.js';

const pkg = JSON.parse(readFileSync(new URL('./package.json', import.meta.url), 'utf8'))

buildProtobuf('../protos/', 'src/protos');

export default [
	{
		input: 'src/main.ts',
		output: {
			file: 'dist/js/main.js',
			format: 'iife'
		},
		plugins: [
			sass({ output: 'dist/css/style.css' }),
			copy({
				targets: [
					{
						src: ['static/*', '!*.html'],
						dest: 'dist'
					},
					{
						src: 'static/*.html',
						dest: 'dist',
						transform: contents => contents.toString().replaceAll('{version}', pkg.version)
					}
				]
			}),
			typescript()
		]
	}
];