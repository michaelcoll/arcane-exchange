<script setup lang="ts">
import type { TradeStatus, TradeTone } from '~/utils/trade';

const props = defineProps<{
  status: TradeStatus;
  /** Pseudo du partenaire, affiché dans les messages. */
  counterparty: string;
  /** L'utilisateur courant a accepté l'échange. */
  accepted: boolean;
  /** L'utilisateur courant a confirmé l'échange physique. */
  confirmed: boolean;
  /** L'abandon vient de l'utilisateur courant (et non du partenaire). */
  abandonedByMe: boolean;
}>();

const content = computed<{ tone: TradeTone; icon: string; title: string; text: string }>(() => {
  const them = props.counterparty;

  switch (props.status) {
    case 'ONE_ACCEPTED':
      return props.accepted
        ? {
            tone: 'cyan',
            icon: 'lucide:lock',
            title: `En attente de ${them}`,
            text: `Tu as accepté — les cartes sont réservées. ${them} doit accepter à son tour, ou tu peux encore modifier.`,
          }
        : {
            tone: 'cyan',
            icon: 'lucide:lock',
            title: `${them} a accepté — à toi de jouer`,
            text: 'Accepte à ton tour pour verrouiller, ou modifie la transaction pour relancer la négociation.',
          };
    case 'FULLY_ACCEPTED':
      return {
        tone: 'violet',
        icon: 'lucide:lock',
        title: 'Transaction verrouillée',
        text: props.confirmed
          ? `Tu as confirmé l'échange. En attente de la confirmation de ${them}.`
          : 'Les deux parties ont accepté. Procédez à l’échange physique en personne, puis confirme ci-dessous.',
      };
    case 'COMPLETED':
      return {
        tone: 'good',
        icon: 'lucide:check',
        title: 'Échange réalisé',
        text: 'Les deux parties ont confirmé l’échange physique. Tu peux noter ton partenaire (optionnel).',
      };
    case 'CLOSED':
      return {
        tone: 'good',
        icon: 'lucide:check',
        title: 'Transaction clôturée',
        text: 'Les notations ont été enregistrées. Merci d’utiliser Arcane Exchange !',
      };
    case 'ABANDONED':
      return {
        tone: 'down',
        icon: 'lucide:x',
        title: 'Échange abandonné',
        text: props.abandonedByMe
          ? 'Tu as abandonné cet échange. Les cartes réservées ont été libérées.'
          : `${them} a abandonné l’échange. Les cartes réservées ont été libérées.`,
      };
    default:
      return {
        tone: 'cyan',
        icon: 'lucide:arrow-left-right',
        title: 'Négociation ouverte',
        text: `Ajoute ou retire des cartes des deux côtés. Chaque modification notifie ${them}.`,
      };
  }
});

const bannerClasses: Record<TradeTone, string> = {
  cyan: 'border-slate-200 bg-slate-100 dark:border-white/15 dark:bg-white/5',
  violet: 'border-violet-500/30 bg-violet-500/10 dark:border-violet-400/30 dark:bg-violet-400/10',
  good: 'border-emerald-500/30 bg-emerald-500/10 dark:border-emerald-400/30 dark:bg-emerald-400/10',
  down: 'border-red-500/30 bg-red-500/10 dark:border-red-400/30 dark:bg-red-400/10',
  muted: 'border-slate-200 bg-slate-100 dark:border-white/15 dark:bg-white/5',
};

const iconClasses: Record<TradeTone, string> = {
  cyan: 'border-cyan-500/30 bg-cyan-500/10 text-cyan-600 dark:border-cyan-400/30 dark:bg-cyan-400/10 dark:text-cyan-400',
  violet:
    'border-violet-500/40 bg-violet-500/20 text-violet-600 dark:border-violet-400/40 dark:bg-violet-400/20 dark:text-violet-300',
  good: 'border-emerald-500/40 bg-transparent text-emerald-600 dark:border-emerald-400/40 dark:text-emerald-400',
  down: 'border-red-500/40 bg-transparent text-red-600 dark:border-red-400/40 dark:text-red-400',
  muted: 'border-slate-200 bg-slate-100 text-slate-500 dark:border-white/15 dark:bg-white/5',
};
</script>

<template>
  <div :class="['flex items-start gap-3 rounded-xl border p-4', bannerClasses[content.tone]]">
    <span
      :class="[
        'grid h-[34px] w-[34px] flex-none place-items-center rounded-lg border',
        iconClasses[content.tone],
      ]"
    >
      <Icon :name="content.icon" size="17" />
    </span>
    <div class="flex min-w-0 flex-col gap-0.5">
      <span class="text-sm font-semibold text-slate-800 dark:text-slate-100">{{
        content.title
      }}</span>
      <span class="text-xs leading-relaxed text-slate-600 dark:text-slate-300">{{
        content.text
      }}</span>
    </div>
  </div>
</template>
