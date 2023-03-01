import { readFileSync } from 'fs';
import copy from 'rollup-plugin-copy';
import sass from 'rollup-plugin-sass';

const pkg = JSON.parse(readFileSync(new URL('./package.json', import.meta.url), 'utf8'))

export default [
	{
		input: 'src/main.js',
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
			})
		]
	}
];