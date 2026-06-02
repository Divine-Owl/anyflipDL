<!--
  Phosphor icon set — Regular weight, inline SVG.
  256x256 viewBox, fill currentColor (filled icons) or stroke currentColor (line icons).
  Usage: <Icon name="download" size={20} />
-->
<script lang="ts" module>
  export type IconName =
    | 'link'
    | 'arrow-right'
    | 'arrow-back'
    | 'download'
    | 'pause'
    | 'play'
    | 'x'
    | 'trash'
    | 'folder-open'
    | 'external-link'
    | 'file-pdf'
    | 'book-open'
    | 'lock'
    | 'gear'
    | 'archive'
    | 'info'
    | 'check'
    | 'warning'
    | 'bell'
    | 'keyboard'
    | 'sliders'
    | 'palette'
    | 'moon'
    | 'sun'
    | 'monitor'
    | 'circle-dot'
    | 'refresh'
    | 'add-box'
    | 'view-list'
    | 'history'
    | 'settings'
    | 'help'
    | 'crop-square'
    | 'filter-none'
    | 'minus'
    | 'coffee';
</script>

<script lang="ts">
  interface Props {
    name: IconName;
    size?: number;
    class?: string;
  }

  let { name, size = 20, class: className = '' }: Props = $props();
</script>

<svg
  width={size}
  height={size}
  viewBox="0 0 256 256"
  class={className}
  fill="none"
  stroke="currentColor"
  stroke-width="16"
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
>
  {#if name === 'link'}
    <path d="M137.54 186.36a8 8 0 0 1 0 11.31l-8.74 8.74a48 48 0 1 1-67.88-67.88l25.38-25.38a48 48 0 0 1 67.88 0 8 8 0 0 1-11.32 11.32 32 32 0 0 0-45.26 0L72.2 138.55a32 32 0 0 0 45.26 45.26l8.74-8.74a8 8 0 0 1 11.32 0z" />
    <path d="M208 160a48 48 0 0 1-26.84 42.84l-25.38 25.38a48 48 0 1 1-67.88-67.88l8.74-8.74a8 8 0 0 1 11.32 11.32l-8.74 8.74a32 32 0 1 0 45.26 45.26l25.38-25.38A32 32 0 0 0 142.84 160a8 8 0 0 1 11.32-11.32A48 48 0 0 1 208 160z" />
  {:else if name === 'arrow-right'}
    <line x1="40" y1="128" x2="216" y2="128" />
    <polyline points="144,56 216,128 144,200" />
  {:else if name === 'arrow-back'}
    <line x1="216" y1="128" x2="40" y2="128" />
    <polyline points="112,56 40,128 112,200" />
  {:else if name === 'download'}
    <line x1="128" y1="40" x2="128" y2="160" />
    <polyline points="80,112 128,160 176,112" />
    <line x1="40" y1="200" x2="216" y2="200" />
  {:else if name === 'pause'}
    <rect x="80" y="40" width="32" height="176" rx="8" fill="currentColor" stroke="none" />
    <rect x="144" y="40" width="32" height="176" rx="8" fill="currentColor" stroke="none" />
  {:else if name === 'play'}
    <polygon points="80,40 80,216 224,128" fill="currentColor" stroke="currentColor" stroke-linejoin="round" />
  {:else if name === 'x'}
    <line x1="200" y1="56" x2="56" y2="200" />
    <line x1="200" y1="200" x2="56" y2="56" />
  {:else if name === 'trash'}
    <line x1="216" y1="56" x2="40" y2="56" />
    <line x1="104" y1="104" x2="104" y2="168" />
    <line x1="152" y1="104" x2="152" y2="168" />
    <path d="M200 56v152a8 8 0 0 1-8 8H64a8 8 0 0 1-8-8V56" />
    <path d="M168 56V40a16 16 0 0 0-16-16h-48a16 16 0 0 0-16 16v16" />
  {:else if name === 'folder-open'}
    <path d="M40 200V64a16 16 0 0 1 16-16h60.7a7.94 7.94 0 0 1 4.8 1.6L160 80H200a16 16 0 0 1 16 16v32" />
    <path d="M40 200l16-80h192l-16 80a8 8 0 0 1-8 6.4H48a8 8 0 0 1-8-6.4z" fill="currentColor" fill-opacity="0.1" />
  {:else if name === 'external-link'}
    <polyline points="144,80 216,80 216,152" />
    <line x1="216" y1="80" x2="104" y2="192" />
    <path d="M184 128v56a8 8 0 0 1-8 8H48a8 8 0 0 1-8-8V56a8 8 0 0 1 8-8h56" />
  {:else if name === 'file-pdf'}
    <path d="M200 224H56a8 8 0 0 1-8-8V40a8 8 0 0 1 8-8h96l56 56v128a8 8 0 0 1-8 8z" fill="currentColor" fill-opacity="0.1" />
    <polyline points="152,32 152,88 208,88" />
    <text x="128" y="172" text-anchor="middle" font-family="Inter, sans-serif" font-size="48" font-weight="700" fill="currentColor" stroke="none">PDF</text>
  {:else if name === 'book-open'}
    <path d="M128 184a48 48 0 0 0-80-32V56a48 48 0 0 1 80 0z" fill="currentColor" fill-opacity="0.1" />
    <path d="M128 184a48 48 0 0 1 80-32V56a48 48 0 0 0-80 0z" fill="currentColor" fill-opacity="0.1" />
    <path d="M128 184V56" />
    <path d="M128 56a48 48 0 0 0-80 0v96a48 48 0 0 1 80 32" />
    <path d="M128 56a48 48 0 0 1 80 0v96a48 48 0 0 0-80 32" />
  {:else if name === 'lock'}
    <rect x="40" y="96" width="176" height="128" rx="8" fill="currentColor" fill-opacity="0.1" />
    <rect x="40" y="96" width="176" height="128" rx="8" />
    <path d="M88 96V64a40 40 0 0 1 80 0v32" />
    <line x1="128" y1="148" x2="128" y2="176" />
  {:else if name === 'gear'}
    <circle cx="128" cy="128" r="40" />
    <path d="M130.05 206.11h-4l-12.42-27.22a75.8 75.8 0 0 1-22.42-9.43l-27.51 11.43-2.83-2.83 11.46-27.55a75.8 75.8 0 0 1-9.42-22.43L33.61 115.66v-4l27.22-12.45a75.8 75.8 0 0 1 9.43-22.43L58.8 49.32l2.83-2.83L89.18 57.9a75.8 75.8 0 0 1 22.42-9.42L124.05 21h4l12.46 27.27a75.8 75.8 0 0 1 22.42 9.43l27.55-11.46 2.83 2.83-11.46 27.55a75.8 75.8 0 0 1 9.42 22.42l27.21 12.42v4l-27.21 12.46a75.8 75.8 0 0 1-9.42 22.42l11.43 27.51-2.83 2.83-27.51-11.46a75.8 75.8 0 0 1-22.42 9.42z" />
  {:else if name === 'archive'}
    <rect x="32" y="56" width="192" height="160" rx="8" fill="currentColor" fill-opacity="0.1" />
    <line x1="32" y1="104" x2="224" y2="104" />
    <line x1="32" y1="152" x2="120" y2="152" />
    <path d="M216 56H40a8 8 0 0 0-8 8v32h192V64a8 8 0 0 0-8-8z" fill="currentColor" fill-opacity="0.2" />
  {:else if name === 'info'}
    <circle cx="128" cy="128" r="96" />
    <line x1="128" y1="120" x2="128" y2="172" />
    <circle cx="128" cy="84" r="10" fill="currentColor" stroke="none" />
  {:else if name === 'check'}
    <polyline points="40,144 96,200 224,72" />
  {:else if name === 'warning'}
    <path d="M142.86 26.22l98.7 170a16 16 0 0 1-13.79 24H28.23a16 16 0 0 1-13.79-24l98.7-170a16 16 0 0 1 27.42 0z" />
    <line x1="128" y1="104" x2="128" y2="160" />
    <circle cx="128" cy="180" r="10" fill="currentColor" stroke="none" />
  {:else if name === 'bell'}
    <path d="M56 104a72 72 0 0 1 144 0c0 35.5 8 56 24 72H32c16-16 24-36.5 24-72z" />
    <path d="M96 192a32 32 0 0 0 64 0" />
  {:else if name === 'keyboard'}
    <rect x="32" y="56" width="192" height="144" rx="8" />
    <line x1="64" y1="92" x2="64" y2="92" />
    <line x1="96" y1="92" x2="96" y2="92" />
    <line x1="128" y1="92" x2="128" y2="92" />
    <line x1="160" y1="92" x2="160" y2="92" />
    <line x1="192" y1="92" x2="192" y2="92" />
    <line x1="64" y1="132" x2="64" y2="132" />
    <line x1="96" y1="132" x2="96" y2="132" />
    <line x1="128" y1="132" x2="128" y2="132" />
    <line x1="160" y1="132" x2="160" y2="132" />
    <line x1="192" y1="132" x2="192" y2="132" />
    <line x1="72" y1="172" x2="184" y2="172" />
  {:else if name === 'sliders'}
    <line x1="40" y1="80" x2="216" y2="80" />
    <line x1="40" y1="128" x2="216" y2="128" />
    <line x1="40" y1="176" x2="216" y2="176" />
    <circle cx="100" cy="80" r="12" fill="currentColor" stroke="currentColor" />
    <circle cx="160" cy="128" r="12" fill="currentColor" stroke="currentColor" />
    <circle cx="80" cy="176" r="12" fill="currentColor" stroke="currentColor" />
  {:else if name === 'palette'}
    <circle cx="128" cy="128" r="96" />
    <circle cx="80" cy="100" r="14" fill="currentColor" stroke="none" />
    <circle cx="168" cy="92" r="14" fill="currentColor" stroke="none" />
    <circle cx="184" cy="152" r="14" fill="currentColor" stroke="none" />
    <circle cx="128" cy="184" r="14" fill="currentColor" stroke="none" />
    <path d="M128 224c-4 0-8-2-8-7a16 16 0 0 1 16-17h17a24 24 0 0 0 24-24 96 96 0 1 0-192 0 24 24 0 0 0 24 24h17a16 16 0 0 1 16 17c0 5-4 7-8 7z" fill="currentColor" fill-opacity="0.1" />
  {:else if name === 'moon'}
    <path d="M224 152a88 88 0 0 1-119.6 0 88 88 0 1 0 119.6 0z" fill="currentColor" fill-opacity="0.1" />
    <path d="M224 152a88 88 0 0 1-119.6 0 88 88 0 1 0 119.6 0z" />
  {:else if name === 'sun'}
    <circle cx="128" cy="128" r="56" fill="currentColor" fill-opacity="0.1" />
    <circle cx="128" cy="128" r="56" />
    <line x1="128" y1="32" x2="128" y2="48" />
    <line x1="128" y1="208" x2="128" y2="224" />
    <line x1="32" y1="128" x2="48" y2="128" />
    <line x1="208" y1="128" x2="224" y2="128" />
    <line x1="60" y1="60" x2="72" y2="72" />
    <line x1="184" y1="184" x2="196" y2="196" />
    <line x1="60" y1="196" x2="72" y2="184" />
    <line x1="184" y1="72" x2="196" y2="60" />
  {:else if name === 'monitor'}
    <rect x="32" y="48" width="192" height="128" rx="8" fill="currentColor" fill-opacity="0.1" />
    <rect x="32" y="48" width="192" height="128" rx="8" />
    <line x1="96" y1="208" x2="160" y2="208" />
    <line x1="128" y1="176" x2="128" y2="208" />
  {:else if name === 'circle-dot'}
    <circle cx="128" cy="128" r="96" fill="currentColor" fill-opacity="0.1" />
    <circle cx="128" cy="128" r="96" />
    <circle cx="128" cy="128" r="32" fill="currentColor" stroke="none" />
  {:else if name === 'refresh'}
    <polyline points="192,72 224,72 224,40" />
    <path d="M205.3 152A88 88 0 1 1 217 104" />
  {:else if name === 'add-box'}
    <rect x="40" y="40" width="176" height="176" rx="8" fill="currentColor" fill-opacity="0.1" />
    <rect x="40" y="40" width="176" height="176" rx="8" />
    <line x1="88" y1="128" x2="168" y2="128" />
    <line x1="128" y1="88" x2="128" y2="168" />
  {:else if name === 'view-list'}
    <line x1="80" y1="64" x2="216" y2="64" />
    <line x1="80" y1="128" x2="216" y2="128" />
    <line x1="80" y1="192" x2="216" y2="192" />
    <circle cx="48" cy="64" r="8" fill="currentColor" stroke="none" />
    <circle cx="48" cy="128" r="8" fill="currentColor" stroke="none" />
    <circle cx="48" cy="192" r="8" fill="currentColor" stroke="none" />
  {:else if name === 'history'}
    <polyline points="80,72 32,128 80,184" />
    <path d="M32 128h144a48 48 0 0 1 0 96H96" />
  {:else if name === 'settings'}
    <circle cx="128" cy="128" r="40" />
    <path d="M130.05 206.11h-4l-12.42-27.22a75.8 75.8 0 0 1-22.42-9.43l-27.51 11.43-2.83-2.83 11.46-27.55a75.8 75.8 0 0 1-9.42-22.43L33.61 115.66v-4l27.22-12.45a75.8 75.8 0 0 1 9.43-22.43L58.8 49.32l2.83-2.83L89.18 57.9a75.8 75.8 0 0 1 22.42-9.42L124.05 21h4l12.46 27.27a75.8 75.8 0 0 1 22.42 9.43l27.55-11.46 2.83 2.83-11.46 27.55a75.8 75.8 0 0 1 9.42 22.42l27.21 12.42v4l-27.21 12.46a75.8 75.8 0 0 1-9.42 22.42l11.43 27.51-2.83 2.83-27.51-11.46a75.8 75.8 0 0 1-22.42 9.42z" />
  {:else if name === 'help'}
    <circle cx="128" cy="128" r="96" />
    <path d="M104 96a24 24 0 1 1 33 22.4c-5 2.6-9 8.6-9 14.6v8" />
    <circle cx="128" cy="172" r="10" fill="currentColor" stroke="none" />
  {:else if name === 'crop-square'}
    <rect x="48" y="48" width="160" height="160" rx="8" />
  {:else if name === 'filter-none'}
    <rect x="48" y="48" width="160" height="160" />
  {:else if name === 'minus'}
    <line x1="48" y1="128" x2="208" y2="128" />
  {:else if name === 'coffee'}
    <path d="M86 216a24 24 0 0 1-24-24V96a8 8 0 0 1 8-8h112a8 8 0 0 1 8 8v8h16a24 24 0 0 1 24 24v16a24 24 0 0 1-24 24h-16.8A48 48 0 0 1 160 216H86z" fill="currentColor" fill-opacity="0.1" />
    <path d="M86 216a24 24 0 0 1-24-24V96" />
    <line x1="62" y1="88" x2="194" y2="88" />
    <path d="M190 120h16a16 16 0 0 1 16 16v0a16 16 0 0 1-16 16h-16" />
    <line x1="86" y1="64" x2="86" y2="40" />
    <line x1="118" y1="64" x2="118" y2="32" />
    <line x1="150" y1="64" x2="150" y2="40" />
  {/if}
</svg>
