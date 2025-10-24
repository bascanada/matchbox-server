<script>
  export let pubkey = '';
  export let start = 8; // chars to show at start
  export let end = 4; // chars to show at end

  const short = () => {
    if (!pubkey) return '';
    if (pubkey.length <= start + end + 3) return pubkey;
    return `${pubkey.slice(0, start)}...${pubkey.slice(-end)}`;
  };

  // copyFull removed — copy via select + Ctrl/Cmd+C is available
</script>

<span class="pubkey-component">
  <input
    type="text"
    readonly
    class="pubkey-display"
    value={pubkey}
    title={pubkey}
    on:click={(e) => e.target.select()}
    aria-label="Public key"
  />
</span>

<style>
  .pubkey-component {
    display: inline-flex;
    gap: 8px;
    align-items: center;
  }
  .pubkey-display {
    font-family: monospace;
    font-size: 12px;
    background-color: white;
    word-break: break-all;
    border: 1px solid #e1e4e8;
    padding: 6px 8px;
    border-radius: 4px;
    min-width: 120px;
  }
  /* No copy button — users can click the input to select and copy */
</style>
