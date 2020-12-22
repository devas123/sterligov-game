import '@fortawesome/fontawesome-free/js/fontawesome'
import '@fortawesome/fontawesome-free/js/brands'
import '@fortawesome/fontawesome-free/js/solid'
import App from './App.svelte';


const app = new App({
	target: document.body,
	hydrate: true
});

export default app;