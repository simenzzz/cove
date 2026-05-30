<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { page } from '$app/state';
  import ChannelList from '$components/ChannelList.svelte';
  import { auth } from '$stores/auth';
  import { fetchChannels } from '$stores/channels';
  import { wsClient, type WsMessage } from '$lib/ws/client';
  import { env } from '$env/dynamic/public';
  import UserAvatar from '$components/UserAvatar.svelte';
  import Button from '$components/ui/Button.svelte';
  import { Mic, MicOff, PhoneOff, Phone, Radio } from '@lucide/svelte';

  interface VoiceParticipant {
    user_id: string;
    username: string;
  }

  const serverId = $derived(page.params.serverId ?? '');
  const channelId = $derived(page.params.channelId ?? '');

  let localStream: MediaStream | null = $state(null);
  let participants: VoiceParticipant[] = $state([]);
  let error = $state<string | null>(null);
  let muted = $state(false);
  let joined = $state(false);
  let audioHost: HTMLDivElement | undefined = $state();

  const peers = new Map<string, RTCPeerConnection>();
  const cleanups: Array<() => void> = [];

  function iceServers(): RTCIceServer[] {
    const raw = env.PUBLIC_RTC_ICE_SERVERS?.trim();
    if (!raw) return [{ urls: 'stun:stun.l.google.com:19302' }];
    try {
      const parsed = JSON.parse(raw) as RTCIceServer[];
      return Array.isArray(parsed) ? parsed : [{ urls: raw }];
    } catch {
      return [{ urls: raw }];
    }
  }

  function sendSignal(toUserId: string, signal: unknown) {
    wsClient.send({
      v: 1,
      type: 'voice_signal',
      channel_id: channelId,
      to_user_id: toUserId,
      signal,
    });
  }

  function attachAudio(userId: string, stream: MediaStream) {
    if (!audioHost || audioHost.querySelector(`[data-user-id="${userId}"]`)) return;
    const audio = document.createElement('audio');
    audio.dataset.userId = userId;
    audio.autoplay = true;
    audio.setAttribute('playsinline', 'true');
    audio.srcObject = stream;
    audioHost.append(audio);
  }

  function removeAudio(userId: string) {
    audioHost?.querySelector(`[data-user-id="${userId}"]`)?.remove();
  }

  async function createPeer(userId: string, makeOffer: boolean): Promise<RTCPeerConnection> {
    const existing = peers.get(userId);
    if (existing) return existing;

    const pc = new RTCPeerConnection({ iceServers: iceServers() });
    peers.set(userId, pc);
    localStream?.getTracks().forEach((track) => pc.addTrack(track, localStream!));

    pc.onicecandidate = (event) => {
      if (event.candidate) sendSignal(userId, { type: 'ice', candidate: event.candidate });
    };
    pc.ontrack = (event) => {
      const [stream] = event.streams;
      if (stream) attachAudio(userId, stream);
    };

    if (makeOffer) {
      const offer = await pc.createOffer();
      await pc.setLocalDescription(offer);
      sendSignal(userId, { type: 'offer', sdp: offer });
    }
    return pc;
  }

  async function handleSignal(msg: WsMessage) {
    const from = String(msg.from_user_id ?? '');
    if (!from || from === $auth.user?.id) return;
    const signal = msg.signal as { type?: string; sdp?: RTCSessionDescriptionInit; candidate?: RTCIceCandidateInit };
    const pc = await createPeer(from, false);
    if (signal.type === 'offer' && signal.sdp) {
      await pc.setRemoteDescription(signal.sdp);
      const answer = await pc.createAnswer();
      await pc.setLocalDescription(answer);
      sendSignal(from, { type: 'answer', sdp: answer });
    } else if (signal.type === 'answer' && signal.sdp) {
      await pc.setRemoteDescription(signal.sdp);
    } else if (signal.type === 'ice' && signal.candidate) {
      await pc.addIceCandidate(signal.candidate);
    }
  }

  async function joinVoice() {
    error = null;
    try {
      localStream = await navigator.mediaDevices.getUserMedia({ audio: true, video: false });
      wsClient.send({ v: 1, type: 'voice_join', channel_id: channelId });
      joined = true;
    } catch (e) {
      error = e instanceof Error ? e.message : 'Could not join voice.';
    }
  }

  function leaveVoice() {
    wsClient.send({ v: 1, type: 'voice_leave', channel_id: channelId });
    for (const pc of peers.values()) pc.close();
    peers.clear();
    localStream?.getTracks().forEach((track) => track.stop());
    localStream = null;
    participants = [];
    joined = false;
    audioHost?.replaceChildren();
  }

  function toggleMute() {
    muted = !muted;
    localStream?.getAudioTracks().forEach((track) => {
      track.enabled = !muted;
    });
  }

  onMount(async () => {
    if (serverId) await fetchChannels(serverId);
    cleanups.push(
      wsClient.on('voice_state', (msg) => {
        if (msg.channel_id !== channelId) return;
        participants = (msg.participants as VoiceParticipant[]) ?? [];
        for (const participant of participants) {
          if (participant.user_id !== $auth.user?.id) void createPeer(participant.user_id, false);
        }
      }),
      wsClient.on('voice_user_joined', (msg) => {
        if (msg.channel_id !== channelId) return;
        const userId = String(msg.user_id ?? '');
        if (!userId || userId === $auth.user?.id) return;
        participants = [
          ...participants.filter((p) => p.user_id !== userId),
          { user_id: userId, username: String(msg.username ?? userId) },
        ];
        void createPeer(userId, true);
      }),
      wsClient.on('voice_user_left', (msg) => {
        if (msg.channel_id !== channelId) return;
        const userId = String(msg.user_id ?? '');
        participants = participants.filter((p) => p.user_id !== userId);
        peers.get(userId)?.close();
        peers.delete(userId);
        removeAudio(userId);
      }),
      wsClient.on('voice_signal', (msg) => {
        if (msg.channel_id === channelId) void handleSignal(msg);
      }),
      wsClient.on('voice_error', (msg) => {
        if (msg.channel_id === channelId) error = `${msg.code}: ${msg.message}`;
      }),
      wsClient.onReady(() => {
        if (joined) wsClient.send({ v: 1, type: 'voice_join', channel_id: channelId });
      }),
    );
  });

  onDestroy(() => {
    leaveVoice();
    for (const off of cleanups) off();
  });
</script>

<div class="flex h-full">
  <ChannelList {serverId} />
  <div class="min-w-0 flex-1 overflow-y-auto p-6">
    <div class="mx-auto flex max-w-3xl flex-col gap-5">
      <div class="flex items-center gap-3">
        <span class="flex h-11 w-11 items-center justify-center rounded-2xl bg-teal-soft text-teal-bright">
          <Radio size={20} />
        </span>
        <div>
          <h1 class="font-display text-xl font-semibold text-linen">Voice channel</h1>
          <p class="text-sm text-linen-muted">
            {participants.length} connected{joined ? ' · you’re live' : ''}
          </p>
        </div>
      </div>

      {#if error}
        <div class="rounded-xl border border-danger/40 bg-danger-soft px-3.5 py-3 text-sm text-danger">
          {error}
        </div>
      {/if}

      {#if participants.length > 0}
        <div class="grid grid-cols-2 gap-3 sm:grid-cols-3">
          {#each participants as participant (participant.user_id)}
            {@const isSelf = participant.user_id === $auth.user?.id}
            <div
              class="flex flex-col items-center gap-2.5 rounded-2xl border border-line bg-surface p-5 {isSelf
                ? 'ring-1 ring-teal/40'
                : ''}"
            >
              <div class="relative">
                <UserAvatar username={participant.username} size="lg" />
                {#if isSelf && muted}
                  <span class="absolute -bottom-1 -right-1 flex h-6 w-6 items-center justify-center rounded-full bg-danger text-canvas">
                    <MicOff size={13} />
                  </span>
                {/if}
              </div>
              <span class="truncate text-sm font-medium text-linen">{participant.username}</span>
              {#if isSelf}
                <span class="text-2xs font-semibold uppercase tracking-wide text-teal-bright">You</span>
              {/if}
            </div>
          {/each}
        </div>
      {:else}
        <div class="flex flex-col items-center rounded-2xl border border-dashed border-line-strong bg-surface/40 px-6 py-16 text-center">
          <p class="font-display text-lg font-semibold text-linen">Nobody here yet</p>
          <p class="mt-1 text-sm text-linen-muted">Join the channel to start talking.</p>
        </div>
      {/if}

      <div class="flex gap-2.5">
        {#if joined}
          <Button variant="outline" onclick={toggleMute}>
            {#if muted}<MicOff size={16} /> Unmute{:else}<Mic size={16} /> Mute{/if}
          </Button>
          <Button variant="danger" onclick={leaveVoice}>
            <PhoneOff size={16} /> Leave
          </Button>
        {:else}
          <Button variant="secondary" onclick={joinVoice}>
            <Phone size={16} /> Join voice
          </Button>
        {/if}
      </div>
      <div bind:this={audioHost}></div>
    </div>
  </div>
</div>
