import sass from 'rollup-plugin-sass';

export default [
	{
		input: 'src/main.js',
		output: {
			file: '../public/js/main.js',
			format: 'iife'
		},
		plugins: [
			sass({ output: '../public/style.css' })
		]
	}
];