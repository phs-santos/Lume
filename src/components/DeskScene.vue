<script setup lang="ts">
import type { TimeOfDay } from "../lib/timeOfDay";

withDefaults(defineProps<{ time: TimeOfDay; crop?: "bottom" | "middle" }>(), { crop: "bottom" });
</script>

<!--
  Escrivaninha ilustrada. As cores vêm de variáveis por horário (classe no <svg>),
  então a mesma cena serve de manhã, à tarde e à noite.
-->
<template>
  <svg
    :class="['scene', time]"
    viewBox="0 0 1600 1000"
    :preserveAspectRatio="crop === 'middle' ? 'xMinYMid slice' : 'xMinYMax slice'"
    role="img"
    aria-label="Uma escrivaninha com luminária, janela e folhas de papel"
  >
    <defs>
      <radialGradient id="glow" cx="50%" cy="0%" r="100%">
        <stop offset="0%" stop-color="#ffe7ad" stop-opacity="0.55" />
        <stop offset="60%" stop-color="#ffe7ad" stop-opacity="0.12" />
        <stop offset="100%" stop-color="#ffe7ad" stop-opacity="0" />
      </radialGradient>
      <mask id="crescent">
        <circle cx="300" cy="100" r="42" fill="#fff" />
        <circle cx="320" cy="86" r="38" fill="#000" />
      </mask>
      <linearGradient id="sky" x1="0" y1="0" x2="0" y2="1">
        <stop offset="0%" class="sky-top" />
        <stop offset="100%" class="sky-bottom" />
      </linearGradient>
    </defs>

    <!-- parede -->
    <rect width="1600" height="1000" class="wall" />
    <rect y="0" width="1600" height="40" class="wall-shade" />

    <!-- janela -->
    <g transform="translate(560 110)">
      <rect width="420" height="380" rx="6" class="frame" />
      <rect x="18" y="18" width="384" height="344" rx="2" fill="url(#sky)" />
      <circle v-if="time === 'noite'" cx="300" cy="100" r="42" class="moon" mask="url(#crescent)" />
      <circle v-else cx="290" cy="120" r="46" class="sun" />
      <g v-if="time === 'noite'" class="stars">
        <circle cx="70" cy="70" r="2.5" /><circle cx="150" cy="140" r="2" /><circle cx="220" cy="60" r="2" />
        <circle cx="110" cy="250" r="2" /><circle cx="360" cy="230" r="2.5" /><circle cx="60" cy="180" r="1.6" />
      </g>
      <!-- telhados ao longe -->
      <path d="M18 362 V300 h50 v-30 h40 v30 h30 v-60 h60 v60 h40 v-20 h50 v20 h40 v-45 h60 v105 Z" class="city" />
      <g v-if="time === 'noite'" class="lit-windows">
        <rect x="160" y="272" width="10" height="12" /><rect x="180" y="300" width="10" height="12" />
        <rect x="340" y="295" width="10" height="12" />
      </g>
      <rect x="202" y="18" width="16" height="344" class="frame" />
      <rect x="18" y="182" width="384" height="16" class="frame" />
      <!-- cortina -->
      <path d="M-120 -30 h120 c-14 150 -4 300 26 430 h-146 Z" class="curtain" />
    </g>

    <!-- quadro na parede -->
    <g transform="translate(110 190)">
      <rect width="170" height="210" rx="4" class="frame" />
      <rect x="14" y="14" width="142" height="182" class="art-bg" />
      <path d="M14 160 C 50 110, 90 150, 156 90 V196 H14 Z" class="art-hill" />
      <circle cx="110" cy="62" r="18" class="art-sun" />
    </g>

    <!-- luz da luminária (só à noite) -->
    <path v-if="time === 'noite'" d="M352 468 L540 418 L860 800 H200 Z" fill="url(#glow)" class="lamp-light" transform="translate(-120 0)" />

    <!-- mesa -->
    <rect x="0" y="800" width="1600" height="200" class="desk" />
    <rect x="0" y="790" width="1600" height="22" rx="4" class="desk-top" />

    <!-- luminária -->
    <g class="lamp" transform="translate(-120 0)">
      <rect x="250" y="772" width="180" height="24" rx="12" class="lamp-metal" />
      <path d="M340 774 L300 590 L410 430" class="lamp-arm" />
      <circle cx="300" cy="590" r="13" class="lamp-metal" />
      <path d="M392 368 L452 346 L540 418 L352 468 Z" class="lamp-shade" />
      <path v-if="time === 'noite'" d="M352 468 L540 418" class="bulb-line" />
    </g>

    <!-- folhas na mesa -->
    <g transform="rotate(-5 600 770)">
      <rect x="500" y="690" width="230" height="110" rx="3" class="paper paper-under" />
    </g>
    <g transform="rotate(3 640 760)">
      <rect x="530" y="680" width="230" height="120" rx="3" class="paper" />
      <rect x="555" y="705" width="150" height="7" rx="3" class="ink typed l1" />
      <rect x="555" y="725" width="180" height="7" rx="3" class="ink typed l2" />
      <rect x="555" y="745" width="120" height="7" rx="3" class="ink typed l3" />
      <rect x="544" y="762" width="3" height="18" class="ribbon caret" />
      <rect x="555" y="765" width="80" height="7" rx="3" class="ink typed l4" />
    </g>

    <!-- pilha de livros -->
    <g transform="translate(870 690) scale(0.85)" class="books">
      <rect x="0" y="70" width="230" height="34" rx="3" class="book b1" />
      <rect x="18" y="38" width="200" height="32" rx="3" class="book b2" />
      <rect x="6" y="8" width="214" height="30" rx="3" class="book b3" />
    </g>

    <!-- xícara com vapor -->
    <g transform="translate(790 712) scale(0.9)">
      <path d="M0 20 h80 v50 a30 30 0 0 1 -30 30 h-20 a30 30 0 0 1 -30 -30 Z" class="cup" />
      <path d="M80 34 a18 18 0 0 1 0 36" class="cup-handle" />
      <path class="steam s1" d="M25 10 c-10 -20 10 -30 0 -50" />
      <path class="steam s2" d="M50 6 c-10 -20 10 -30 0 -54" />
    </g>
  </svg>
</template>

<style scoped>
.scene { width: 100%; height: 100%; display: block; }

.noite {
  --wall: #1b2340; --wall-shade: #161d36; --frame: #2a3460; --sky-top: #0f1530; --sky-bottom: #2b3a72;
  --city: #121831; --desk: #2a1f1a; --desk-top: #3b2c24; --paper: #e8ebee; --paper-under: #c9ced6;
  --curtain: #5a2a3a; --art-bg: #2c3766; --art-hill: #3f4c85; --art-sun: #f3e3b5;
  --cup: #e8ebee; --steam: rgba(232, 235, 238, 0.35); --metal: #3a4370;
  --b1: #b42318; --b2: #2f5d50; --b3: #a8742a;
}
.tarde {
  --wall: #e9ddd0; --wall-shade: #dccdbd; --frame: #c4ad96; --sky-top: #f0a26b; --sky-bottom: #f7d39b;
  --city: #b07c62; --desk: #7a5641; --desk-top: #946a50; --paper: #fbfbfb; --paper-under: #e4e4e4;
  --curtain: #3f6f6a; --art-bg: #f0d9b5; --art-hill: #c98b5f; --art-sun: #b42318;
  --cup: #ffffff; --steam: rgba(120, 90, 70, 0.25); --metal: #5b4334;
  --b1: #1c2333; --b2: #2f5d50; --b3: #b42318;
}
.manha {
  --wall: #e4ebf2; --wall-shade: #d6dfe9; --frame: #b9c6d4; --sky-top: #8fc7ea; --sky-bottom: #d9eef8;
  --city: #9fb3c6; --desk: #8a6a52; --desk-top: #a5826a; --paper: #ffffff; --paper-under: #e6e9ee;
  --curtain: #2f5d50; --art-bg: #dbe8f1; --art-hill: #7fa596; --art-sun: #f2b33d;
  --cup: #ffffff; --steam: rgba(90, 110, 130, 0.25); --metal: #6a7a8c;
  --b1: #b42318; --b2: #1c2333; --b3: #a8742a;
}

.wall { fill: var(--wall); }
.wall-shade { fill: var(--wall-shade); }
.frame { fill: var(--frame); }
.sky-top { stop-color: var(--sky-top); }
.sky-bottom { stop-color: var(--sky-bottom); }
.city { fill: var(--city); }
.moon { fill: #f3e3b5; }
.sun { fill: #ffd77a; }
.stars circle, .lit-windows rect { fill: #f3e3b5; }
.curtain { fill: var(--curtain); }
.art-bg { fill: var(--art-bg); }
.art-hill { fill: var(--art-hill); }
.art-sun { fill: var(--art-sun); }
.desk { fill: var(--desk); }
.desk-top { fill: var(--desk-top); }
.paper { fill: var(--paper); }
.paper-under { fill: var(--paper-under); }
.ink { fill: #1c2333; opacity: 0.75; }
.ribbon { fill: #b42318; }
.lamp-metal { fill: var(--metal); }
.lamp-arm { fill: none; stroke: var(--metal); stroke-width: 14; stroke-linecap: round; stroke-linejoin: round; }
.lamp-shade { fill: #b42318; }
.bulb-line { stroke: #fff3cf; stroke-width: 6; stroke-linecap: round; }
.book { stroke: rgba(0, 0, 0, 0.15); stroke-width: 2; }
.b1 { fill: var(--b1); } .b2 { fill: var(--b2); } .b3 { fill: var(--b3); }
.cup { fill: var(--cup); }
.cup-handle { fill: none; stroke: var(--cup); stroke-width: 10; }
.steam { fill: none; stroke: var(--steam); stroke-width: 6; stroke-linecap: round; animation: rise 4s ease-in-out infinite; }
.s2 { animation-delay: 2s; }

@keyframes rise {
  0% { opacity: 0; transform: translateY(8px); }
  40% { opacity: 1; }
  100% { opacity: 0; transform: translateY(-18px); }
}
/* Ao abrir: as linhas da folha são "datilografadas" uma a uma, a fita pisca como cursor,
   a luminária acende (à noite) e a cortina balança devagar. Um momento só, nada chamativo. */
.typed { transform-box: fill-box; transform-origin: left center; transform: scaleX(0); animation: type 0.9s steps(18, end) forwards; }
.l1 { animation-delay: 0.5s; }
.l2 { animation-delay: 1.5s; }
.l3 { animation-delay: 2.5s; }
.l4 { animation-delay: 3.4s; animation-duration: 0.6s; }
@keyframes type { to { transform: scaleX(1); } }
.caret { animation: blink 1.1s steps(1, end) infinite; }
@keyframes blink { 50% { opacity: 0; } }
.lamp-light, .bulb-line { opacity: 0; animation: lamp-on 1.2s ease-out 0.3s forwards; }
@keyframes lamp-on { 0% { opacity: 0; } 18% { opacity: 0.8; } 26% { opacity: 0.15; } 40% { opacity: 1; } 100% { opacity: 1; } }
.curtain { transform-box: fill-box; transform-origin: top center; animation: sway 7s ease-in-out infinite; }
@keyframes sway { 0%, 100% { transform: skewX(0deg); } 50% { transform: skewX(-1.6deg); } }

@media (prefers-reduced-motion: reduce) {
  .steam, .typed, .caret, .lamp-light, .bulb-line, .curtain { animation: none; }
  .typed { transform: none; }
  .lamp-light, .bulb-line { opacity: 1; }
}
</style>
