<script lang="ts">
  import Logo from '$components/ui/Logo.svelte';
  import { MessagesSquare, Radio, MonitorPlay, Feather } from '@lucide/svelte';

  let { children } = $props();

  const highlights = [
    { icon: MessagesSquare, text: 'Channels & DMs that feel like home' },
    { icon: Radio, text: 'Drop into voice in a single tap' },
    { icon: MonitorPlay, text: 'Watch together, perfectly in sync' },
    { icon: Feather, text: 'Co-write docs & whiteboards live' },
  ];
</script>

<div class="flex min-h-screen">
  <!-- Brand panel -->
  <aside
    class="relative hidden w-[46%] flex-col justify-between overflow-hidden border-r border-line bg-surface/40 p-12 lg:flex"
  >
    <!-- layered warm glow -->
    <div
      class="pointer-events-none absolute -left-24 -top-24 h-96 w-96 rounded-full bg-copper/20 blur-3xl"
    ></div>
    <div
      class="pointer-events-none absolute bottom-0 right-0 h-80 w-80 rounded-full bg-teal/15 blur-3xl"
    ></div>

    <div class="relative">
      <Logo size={40} wordmark />
    </div>

    <div class="stagger relative max-w-md">
      <h1 class="font-display text-5xl font-semibold leading-[1.05] tracking-tight text-linen">
        A warmer place
        <span class="text-ember">to gather.</span>
      </h1>
      <p class="mt-5 text-lg leading-relaxed text-linen-dim">
        Cove is where your people hang out — talk, build, and watch together, all in one cozy
        corner of the web.
      </p>

      <ul class="mt-9 space-y-3.5">
        {#each highlights as item}
          {@const Icon = item.icon}
          <li class="flex items-center gap-3 text-linen-dim">
            <span
              class="flex h-9 w-9 shrink-0 items-center justify-center rounded-xl bg-copper-soft text-copper-bright"
            >
              <Icon size={17} />
            </span>
            <span class="text-sm">{item.text}</span>
          </li>
        {/each}
      </ul>
    </div>

    <p class="relative text-xs text-linen-muted">Built for the people you keep coming back to.</p>
  </aside>

  <!-- Form panel -->
  <main class="flex flex-1 items-center justify-center p-6">
    <div class="w-full max-w-sm animate-fade-up">
      <div class="mb-8 lg:hidden">
        <Logo size={34} wordmark />
      </div>
      {@render children()}
    </div>
  </main>
</div>
