<script lang="ts">
  import Icon, { type IconName } from './icons/Icon.svelte';

  interface Props {
    value: string;
    placeholder?: string;
    mono?: boolean;
    disabled?: boolean;
    readonly?: boolean;
    type?: 'text' | 'password' | 'url' | 'search';
    ariaLabel?: string;
    id?: string;
    name?: string;
    trailingIcon?: IconName;
    onTrailingClick?: () => void;
    onInput?: (e: Event) => void;
    onKeydown?: (e: KeyboardEvent) => void;
    onPaste?: (e: ClipboardEvent) => void;
  }

  let {
    value = $bindable(''),
    placeholder = '',
    mono = false,
    disabled = false,
    readonly = false,
    type = 'text',
    ariaLabel,
    id,
    name,
    trailingIcon,
    onTrailingClick,
    onInput,
    onKeydown,
    onPaste,
  }: Props = $props();
</script>

<div class="input-wrap {mono ? 'input-wrap--mono' : ''}" class:input-wrap--disabled={disabled}>
  <input
    class="input"
    {type}
    {placeholder}
    {disabled}
    {readonly}
    {id}
    {name}
    aria-label={ariaLabel}
    aria-readonly={readonly || undefined}
    autocomplete="off"
    spellcheck="false"
    bind:value
    oninput={onInput}
    onkeydown={onKeydown}
    onpaste={onPaste}
  />
  {#if trailingIcon}
    <button
      type="button"
      class="trailing"
      tabindex="-1"
      aria-label="Clear"
      onclick={onTrailingClick}
    >
      <Icon name={trailingIcon} size={16} />
    </button>
  {/if}
</div>

<style>
  .input-wrap {
    display: flex;
    align-items: center;
    width: 100%;
    height: 42px;
    background: var(--color-surface-input);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-control);
    padding: 0 var(--space-3);
    transition: border-color var(--duration-fast) var(--ease-standard);
    position: relative;
  }

  .input-wrap--mono { font-family: var(--font-family-mono); }

  .input-wrap:focus-within {
    outline: var(--focus-ring);
    outline-offset: 0;
  }

  .input-wrap--disabled { opacity: 0.6; }

  .input[readonly] {
    color: var(--color-text-secondary);
    cursor: default;
  }

  .input {
    flex: 1;
    height: 100%;
    background: transparent;
    border: none;
    outline: none;
    font-size: var(--font-size-base);
    line-height: var(--line-height-base);
    color: var(--color-text-primary);
    min-width: 0;
  }

  .input::placeholder { color: var(--color-text-muted); }

  .input-wrap--mono .input {
    font-family: var(--font-family-mono);
  }

  .trailing {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    border: none;
    background: transparent;
    color: var(--color-text-muted);
    border-radius: var(--radius-sm);
    cursor: pointer;
    flex-shrink: 0;
    margin-left: var(--space-2);
  }

  .trailing:hover {
    background: var(--color-neutral-100);
    color: var(--color-text-primary);
  }
</style>
