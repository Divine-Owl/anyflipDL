<script lang="ts">
  import { passwordPromptDownloadId, submitPassword, cancelPasswordPrompt } from '$lib/stores/downloads';
  import RaisedCard from './RaisedCard.svelte';
  import Button from './Button.svelte';
  import Input from './Input.svelte';
  import Icon from './icons/Icon.svelte';

  let value = $state('');
  let revealed = $state(false);
  let isOpen = $derived($passwordPromptDownloadId !== null);

  function onKey(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.preventDefault();
      cancelPasswordPrompt();
    } else if (e.key === 'Enter' && value.length > 0) {
      e.preventDefault();
      submitPassword(value);
      value = '';
    }
  }

  function onBackdropKey(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.preventDefault();
      cancelPasswordPrompt();
    }
  }
</script>

{#if isOpen}
  <div
    class="backdrop"
    role="presentation"
    onclick={cancelPasswordPrompt}
    onkeydown={onBackdropKey}
  >
    <div
      class="modal"
      onclick={(e) => e.stopPropagation()}
      onkeydown={(e) => e.stopPropagation()}
      role="dialog"
      tabindex={-1}
      aria-modal="true"
      aria-labelledby="pwd-title"
    >
      <RaisedCard padding="lg">
        <header>
          <div class="icon"><Icon name="info" size={20} /></div>
          <h3 id="pwd-title">Password required</h3>
        </header>
        <p class="body">This document is protected. Enter the password to continue.</p>
        <div class="row">
          <Input
            bind:value={value}
            type={revealed ? 'text' : 'password'}
            placeholder="Password"
            ariaLabel="Document password"
            onKeydown={onKey}
          />
          <Button
            variant="ghost"
            size="md"
            leadingIcon={revealed ? 'x' : 'view-list'}
            aria-label={revealed ? 'Hide password' : 'Show password'}
            onclick={() => { revealed = !revealed; }}
          ></Button>
        </div>
        <div class="actions">
          <Button variant="ghost" size="md" onclick={cancelPasswordPrompt}>Cancel</Button>
          <Button
            variant="primary"
            size="md"
            disabled={!value}
            onclick={() => { submitPassword(value); value = ''; }}
          >
            Submit
          </Button>
        </div>
      </RaisedCard>
    </div>
  </div>
{/if}

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    background: rgba(15, 23, 42, 0.45);
    z-index: 1400;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: var(--space-6);
    animation: fadeIn var(--duration-base) var(--ease-standard);
  }
  @keyframes fadeIn { from { opacity: 0 } to { opacity: 1 } }
  .modal { width: 100%; max-width: 480px; }
  header { display: flex; align-items: center; gap: var(--space-3); margin-bottom: var(--space-3); }
  .icon {
    width: 36px; height: 36px;
    display: inline-flex; align-items: center; justify-content: center;
    border-radius: var(--radius-chip);
    background: var(--color-primary-50);
    color: var(--color-primary-700);
  }
  h3 {
    font-size: var(--font-size-lg);
    line-height: var(--line-height-lg);
    font-weight: var(--font-weight-semibold);
    color: var(--color-text-primary);
  }
  .body {
    font-size: var(--font-size-sm);
    line-height: var(--line-height-sm);
    color: var(--color-text-secondary);
    margin-bottom: var(--space-4);
  }
  .row { display: flex; gap: var(--space-2); align-items: center; margin-bottom: var(--space-5); }
  .actions { display: flex; justify-content: flex-end; gap: var(--space-2); }
</style>
