<script lang="ts">
    import { createEventDispatcher } from "svelte";
    import { afterUpdate, beforeUpdate, bind, onMount } from "svelte/internal";
import { getColorValue, getContrast } from "./const";
    import { userId } from "./stores";

    export let messages = [];
    export let players_colors: Map<number,number>;
    let dispatch = createEventDispatcher();
    let inputmessage;
    let div;
    let autoscroll;
    beforeUpdate(() => {
        autoscroll =
            div && div.offsetHeight + div.scrollTop > div.scrollHeight - 20;
    });

    afterUpdate(() => {
        if (autoscroll) div.scrollTo(0, div.scrollHeight);
    });
    function handleKeydown(
        event: KeyboardEvent & { currentTarget: EventTarget & Window }
    ) {
        if (event.code === "Enter" && inputmessage && inputmessage.length > 0) {
            sendMessage(inputmessage);
            inputmessage = "";
        }
    }

    onMount(() => {
        div.scrollTo(0, div.scrollHeight);
    });

    function sendMessage(message: string) {
        dispatch("messagesent", {
            message,
        });
    }

    const sendAndReset = () => {
        sendMessage(inputmessage);
        inputmessage = "";
    };
</script>

<style>
    .chat-container {
        display: flex;
        flex-direction: column;
        width: 100%;
        height: 100%;
        min-height: 0;
        overflow: auto;
    }
    .chat-line {
        display: flex;
        flex-direction: column;
        margin: 3px 0;
        background-color: rgb(255, 204, 127);
        border-radius: 1em 1em 1em 0;
        line-height: 1;
        padding: 0 5px;
        max-width: 80%;
        align-self: flex-start;
    }
    .input-line {
        display: block;
        position: relative;
        width: 100%;
    }
    .chat-line.own {
        align-self: flex-end;
    }
    .chat-line.own,
    .chat-line.own *,
    .chat-line.own > * {
        text-align: right;
        border-radius: 1em 1em 0 1em;
    }

    .chat-username {
        font-weight: 500;
        margin-bottom: 2px;
        margin-right: 3px;
    }
    .chat-message {
        margin-bottom: 4px;
        overflow-wrap: break-word;
        word-break: break-all;
        word-wrap: break-word;
    }
    input[type="text"] {
        padding-right: 2em;
        width: 100%;
        margin: 0;
    }
    .embedded {
        position: absolute;
        right: 10px;
        top: 10px;
        cursor: pointer;
    }
</style>

<svelte:window on:keydown={handleKeydown} />

<div class="chat-container" bind:this={div}>
    {#each messages as message}
        <div class="chat-line" class:own={+$userId === +message?.user_id} style="background-color: {getColorValue(players_colors.get(+message?.user_id))}; color: {getContrast(players_colors.get(+message?.user_id))}">
            <div class="chat-username">{message?.by}</div>
            <div class="chat-message">{message?.message}</div>
        </div>
    {/each}
    <div class="fill" />
</div>
<div class="input-line">
    <input
        type="text"
        bind:value={inputmessage}
        placeholder="Write message and press enter"
        on:focus
        on:blur />

    <!-- svelte-ignore a11y-missing-attribute -->
    <a on:click={sendAndReset}><i
        class="embedded fa fa-paper-plane"
        aria-hidden="true"
         /></a>
</div>
