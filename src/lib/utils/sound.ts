/**
 * Sound utilities for download notifications.
 *
 * Uses the Web Audio API to generate a short chime sound without
 * requiring any external audio files.
 *
 * @module $lib/utils/sound
 */

/** Shared AudioContext instance (created lazily on first use). */
let audioCtx: AudioContext | null = null;

/**
 * Play a short, pleasant chime notification.
 *
 * Generates a two-tone ascending chime using the Web Audio API.
 * The sound is designed to be pleasant and non-intrusive.
 *
 * This function is safe to call in SSR environments -- it will
 * silently no-op if `AudioContext` is not available.
 */
export function playNotificationSound(): void {
  if (typeof window === 'undefined' || typeof AudioContext === 'undefined') {
    return;
  }

  try {
    if (!audioCtx) {
      audioCtx = new AudioContext();
    }

    const ctx = audioCtx;
    const now = ctx.currentTime;

    // First tone (E5)
    playTone(ctx, now, 659.25, 0.15, 0.2);
    // Second tone (G#5)
    playTone(ctx, now + 0.12, 830.61, 0.15, 0.2);
    // Third tone (B5) -- slightly louder for resolution
    playTone(ctx, now + 0.24, 987.77, 0.2, 0.15);
  } catch {
    // Audio playback failed silently -- not critical
  }
}

/**
 * Play a single tone using an oscillator node.
 *
 * @param ctx      - The AudioContext to use.
 * @param startTime - When the tone should start playing.
 * @param frequency - The frequency in Hz.
 * @param volume    - Peak gain (0 to 1).
 * @param duration  - Duration in seconds.
 */
function playTone(
  ctx: AudioContext,
  startTime: number,
  frequency: number,
  volume: number,
  duration: number,
): void {
  const oscillator = ctx.createOscillator();
  const gainNode = ctx.createGain();

  oscillator.type = 'sine';
  oscillator.frequency.setValueAtTime(frequency, startTime);

  // Smooth attack and release envelope to avoid clicks
  gainNode.gain.setValueAtTime(0, startTime);
  gainNode.gain.linearRampToValueAtTime(volume, startTime + 0.01);
  gainNode.gain.exponentialRampToValueAtTime(0.001, startTime + duration);

  oscillator.connect(gainNode);
  gainNode.connect(ctx.destination);

  oscillator.start(startTime);
  oscillator.stop(startTime + duration);
}
