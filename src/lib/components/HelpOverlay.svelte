<script lang="ts">
  import { helpOpen, closeHelp } from '$lib/stores/help';
  import { updateSetting } from '$lib/stores/settings';
  import RaisedCard from './RaisedCard.svelte';
  import Icon from './icons/Icon.svelte';

  interface Step {
    title: string;
    description: string;
  }

  const steps: Step[] = [
    {
      title: '1. Paste an Anyflip URL',
      description: 'Open the New tab, paste a link like anyflip.com/user/book, and fetch its metadata.',
    },
    {
      title: '2. Pick a format',
      description: 'Choose PDF or EPUB and rename the file if you like, then add it to the queue.',
    },
    {
      title: '3. Watch the queue',
      description: 'The Queue tab shows live progress. Pause, resume, retry, or cancel from there.',
    },
    {
      title: '4. Open your downloads',
      description: 'Finished files land in History. Open them or jump to the folder in one click.',
    },
  ];

  let dontShowAgain = $state(false);

  function handleClose() {
    if (dontShowAgain) {
      void updateSetting('seenOnboarding', true);
    }
    closeHelp();
  }

  function onBackdropKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.preventDefault();
      handleClose();
    }
  }
</script>

<svelte:window onkeydown={(e) => { if ($helpOpen && e.key === 'Escape') onBackdropKeydown(e); }} />

{#if $helpOpen}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="backdrop" onclick={handleClose} role="presentation">
    <div class="modal-wrap" onclick={(e) => e.stopPropagation()} role="dialog" aria-modal="true" aria-labelledby="help-title" tabindex="-1">
      <RaisedCard padding="lg" class="overlay">
        <div class="head">
          <div class="head-title">
            <Icon name="help" size={18} />
            <h2 id="help-title">Welcome to AnyflipDL</h2>
          </div>
          <button class="close" type="button" aria-label="Close" onclick={handleClose}>
            <Icon name="x" size={18} />
          </button>
        </div>

        <p class="intro">
          A quick tour of how a download flows through the app. You can reopen this help
          any time from the <span class="kbd">?</span> icon in the title bar.
        </p>

        <ol class="steps">
          {#each steps as step (step.title)}
            <li class="step">
              <div class="step-title">{step.title}</div>
              <div class="step-desc">{step.description}</div>
            </li>
          {/each}
        </ol>

        <label class="checkbox">
          <input
            type="checkbox"
            bind:checked={dontShowAgain}
          />
          <span>Don't show this again</span>
        </label>

        <div class="actions">
          <button type="button" class="primary" onclick={handleClose}>
            Got it
          </button>
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
    z-index: 1500;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: var(--space-6);
    animation: fadeIn var(--duration-base) var(--ease-standard);
  }

  @keyframes fadeIn {
    from { opacity: 0; }
    to   { opacity: 1; }
  }

  .modal-wrap {
    width: 100%;
    max-width: 520px;
  }

  :global(.overlay) {
    max-height: 80vh;
    overflow-y: auto;
  }

  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: var(--space-4);
  }

  .head-title {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    color: var(--color-primary-700);
  }

  h2 {
    margin: 0;
    font-size: var(--font-size-xl);
    line-height: var(--line-height-xl);
    font-weight: var(--font-weight-semibold);
    color: var(--color-text-primary);
    letter-spacing: var(--letter-spacing-tight);
  }

  .close {
    width: 32px;
    height: 32px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border: none;
    background: transparent;
    color: var(--color-text-muted);
    border-radius: var(--radius-sm);
    cursor: pointer;
  }
  .close:hover { background: var(--color-neutral-100); color: var(--color-text-primary); }
  .close:focus-visible { outline: var(--focus-ring); outline-offset: 1px; }

  .intro {
    margin: 0 0 var(--space-5);
    font-size: var(--font-size-sm);
    line-height: var(--line-height-sm);
    color: var(--color-text-secondary);
  }

  .kbd {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 20px;
    padding: 0 5px;
    height: 18px;
    font-family: var(--font-family-mono);
    font-size: var(--font-size-2xs);
    color: var(--color-text-primary);
    background: var(--color-surface-secondary);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
  }

  .steps {
    list-style: none;
    margin: 0 0 var(--space-5);
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }

  .step {
    padding: var(--space-3) var(--space-4);
    background: var(--color-surface-secondary);
    border: 1px solid var(--color-border-subtle);
    border-radius: var(--radius-control);
  }

  .step-title {
    font-size: var(--font-size-sm);
    line-height: var(--line-height-sm);
    font-weight: var(--font-weight-semibold);
    color: var(--color-text-primary);
    margin-bottom: 2px;
  }

  .step-desc {
    font-size: var(--font-size-sm);
    line-height: var(--line-height-sm);
    color: var(--color-text-secondary);
  }

  .checkbox {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    font-size: var(--font-size-sm);
    line-height: var(--line-height-sm);
    color: var(--color-text-secondary);
    cursor: pointer;
    user-select: none;
  }
  .checkbox input { cursor: pointer; }

  .actions {
    display: flex;
    justify-content: flex-end;
    margin-top: var(--space-5);
  }

  .primary {
    appearance: none;
    border: none;
    cursor: pointer;
    font: inherit;
    font-weight: var(--font-weight-semibold);
    font-size: var(--font-size-sm);
    line-height: var(--line-height-sm);
    padding: var(--space-2) var(--space-5);
    background: var(--color-primary-500);
    color: var(--color-text-inverse);
    border-radius: var(--radius-control);
    transition: background-color var(--duration-fast) var(--ease-standard);
  }
  .primary:hover { background: var(--color-primary-600); }
  .primary:focus-visible { outline: var(--focus-ring); outline-offset: 1px; }
</style>
