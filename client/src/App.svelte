<!-- App.svelte -->
<script>
  import Router from "svelte-spa-router";
  import { routes } from "./routes";
</script>

<style>  main {
    height: 100%;
    width: 60%;
    text-align: left;
    margin: 0 auto;
    background-color: rgb(221, 198, 167);
    min-height: 600px;
    min-width: 800px;
    padding-left: 1rem;
    padding-right: 1rem;
    border-radius: 3px;
  }

  :global(body) {
    background-color: rgb(226, 178, 114);
    margin: auto !important;
    padding-top: 1em !important;
    display: flex;
    flex-direction: column;
    align-items: center;
  }

  :global(html) {
    height: 100%;
    margin: 0;
    padding: 0;
  }
  .footer {
      display: flex;
      width: auto;
      flex-direction: column;
  }
  .fill {
      flex-grow: 1;
      flex-shrink: 1;
  }
</style>

<main>
  <Router {routes} />
</main>
<div class="footer">
  <div class="fill"></div>
  <span>Source code at <a href="https://github.com/devas123/sterligov-game">github</a></span>
</div>
