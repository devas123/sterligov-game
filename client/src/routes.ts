import Home from './Home.svelte'
import Room from './Room.svelte'

export const routes = {
    // Exact path
    '/': Home,
    // Using named parameters, with last being optional
    '/room/:id': Room,
    // Catch-all
    // This is optional, but if present it must be the last
    '*': Home,
}