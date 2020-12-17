import { writable } from 'svelte/store';
import type { RoomDesc } from './model';

export const userToken = writable(localStorage.getItem("x-user-token"));
export const userName = writable(localStorage.getItem("x-user-name"));
export const createdAt = writable(localStorage.getItem("x-user-id-created-at"));
export const userId = writable(localStorage.getItem("x-user-id"));
export const selectedRoom = writable(<RoomDesc>{});