<script lang="ts">
    import { createEventDispatcher } from "svelte";
    import type { bind } from "svelte/internal";
import { userId } from "./stores";

    export let messages = [];
    let dispatch = createEventDispatcher();

    function handleKeydown(
        event: KeyboardEvent & { currentTarget: EventTarget & Window }
    ) {
        if (event.code === "Enter" && inputmessage && inputmessage.length > 0) {
            sendMessage(inputmessage);
            inputmessage = '';
        }
    }

    function sendMessage(message: string) {
        dispatch("messagesent", {
            message,
        });
    }
    let inputmessage;
</script>

<style>
    .chat-container {
        display: flex;
        flex-direction: column;
        width: 100%;
        height: 100%;
    }
    .chat-line {
        display: flex;
        flex-direction: column;
        margin: 3px 0;
        background-color: rgb(255, 204, 127);
        border-radius: 3px;
        line-height: 1;
        padding: 0 5px;
    }
    .chat-line.own {
        background-color: rgb(163, 124, 65);
    }
    .chat-username {
        font-weight: 500;
        margin-bottom: 2px;
        margin-right: 3px;
    }
    .chat-message {
        margin-bottom: 4px;
        overflow-wrap: anywhere;
    }
</style>
<svelte:window on:keydown={handleKeydown} />

<div class="chat-container">
    {#each messages as message}
        <div class="chat-line" class:own={+$userId === +message?.by}>
            <div class="chat-username">{message?.by}</div>
            <div class="chat-message">{message?.message}</div>
        </div>
    {/each}
    <div class="fill" />
    <input
        type="text"
        bind:value={inputmessage}
        placeholder="Write message and press enter" />
</div>
