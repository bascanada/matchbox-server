import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

export default defineConfig({
	plugins: [sveltekit()],
	optimizeDeps: {
		exclude: ['argon2-browser']
	},
	ssr: {
		noExternal: [],
		external: ['argon2-browser']
	},
	define: {
		global: 'globalThis',
	},
	resolve: {
		alias: {
			buffer: 'buffer',
			process: 'process/browser',
		}
	},
	build: {
		rollupOptions: {
			external: ['argon2-browser']
		}
	}
});
