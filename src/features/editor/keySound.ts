let ctx: AudioContext | null = null;

/** Clique curto e seco, gerado na hora (sem arquivo de áudio). */
export function playKey() {
  ctx ??= new AudioContext();
  const len = Math.floor(ctx.sampleRate * 0.025);
  const buf = ctx.createBuffer(1, len, ctx.sampleRate);
  const d = buf.getChannelData(0);
  for (let i = 0; i < len; i++) d[i] = (Math.random() * 2 - 1) * Math.pow(1 - i / len, 6);
  const src = ctx.createBufferSource();
  const filter = ctx.createBiquadFilter();
  const gain = ctx.createGain();
  filter.type = "bandpass";
  filter.frequency.value = 1800 + Math.random() * 600;
  gain.gain.value = 0.35;
  src.buffer = buf;
  src.connect(filter).connect(gain).connect(ctx.destination);
  src.start();
}
