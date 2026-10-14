<script setup lang="ts">
import type { TradeCard } from '~/bindings/TradeCard';
import type { TradeViewSide } from '~/composables/useTradeView';

/* Un côté de l'échange en « table de jeu », comme `TradeCardRail` côté iOS : un en-tête qui dit
 * qui pose quoi et pour combien, puis les cartes face visible, côte à côte, en grand. */
const props = withDefaults(
  defineProps<{
    /** Username de celui qui pose ces cartes ; `null` pour moi (« JE POSE »). */
    owner: string | null;
    /** `receive` (le partenaire, en secondary) ou `give` (moi, neutre). */
    direction: 'give' | 'receive';
    /** Le côté tel que la vue du Trade l'expose : ses cartes et leur total, déjà calculé. */
    side: TradeViewSide;
    /** Les cartes de l'échange sont réservées. */
    reserved: boolean;
    /** Chaque tuile porte un badge « − » de retrait. */
    removable: boolean;
    /** Une action est en cours : le retrait est désactivé. */
    busy?: boolean;
    /** Porté en `aria-label` par la tuile fantôme d'un côté vide qu'on ne peut pas remplir. */
    emptyMessage: string;
    /** Emplacement pointillé de fin de rail, affiché seulement si fourni. */
    addLabel?: string;
    /** `rail`: the tiles scroll on one line (mobile); `grid`: they wrap onto new lines
     * (desktop). Only the layout changes. */
    layout?: 'rail' | 'grid';
  }>(),
  { addLabel: undefined, layout: 'rail' },
);

const emit = defineEmits<{ remove: [card: TradeCard]; add: []; open: [card: TradeCard] }>();

const valueClass = computed(() =>
  props.direction === 'receive' ? 'text-secondary' : 'text-[var(--ink)]',
);

/* Une carte verrouillée est atténuée, pas masquée : c'est toujours la carte échangée. */
const dimmed = computed(() => props.reserved && !props.removable);

/* In the grid, the tiles take the collection's default ("md") size: they fill columns of at least
 * 185px, and the placeholders keep the card ratio. */
const tileSize = computed(() => (props.layout === 'grid' ? 'w-full' : 'w-[116px]'));
const placeholder = computed(() => [
  'flex flex-none flex-col items-center justify-center gap-1.5 rounded-[10px] border-[1.5px] border-dashed border-[var(--line-2)] px-1.5 text-[var(--ink-3)]',
  props.layout === 'grid' ? 'aspect-[5/7] w-full' : 'h-[162px] w-[116px]',
]);
const badge =
  'absolute -top-[7px] -left-[7px] z-[6] grid h-6 w-6 place-items-center rounded-full ring-2 ring-[var(--bg)]';
</script>

<template>
  <div class="flex flex-col gap-2">
    <!-- As tall as the partner's avatar, so both sides' tiles start on the same line. -->
    <div data-part="header" class="flex min-h-[26px] items-center gap-[9px] px-1">
      <PlayerAvatar v-if="owner" :username="owner" class="!h-[26px] !w-[26px]" />
      <span class="min-w-0 truncate font-mono text-[10.5px] tracking-[0.13em] text-[var(--ink-3)]">
        <span :class="direction === 'receive' ? 'text-secondary' : ''">{{
          owner ? `@${owner.toUpperCase()}` : 'JE'
        }}</span>
        <span class="tracking-[0.06em] text-[var(--ink-2)]"> POSE</span>
      </span>
      <span class="flex-1" />
      <span
        data-part="total"
        :class="['font-mono text-[11px] font-medium whitespace-nowrap tabular-nums', valueClass]"
        >{{ formatPrice(side.total) }}</span
      >
    </div>

    <!-- Le padding haut/gauche laisse la place aux badges qui débordent du coin des tuiles. -->
    <div
      data-part="tiles"
      :data-layout="layout"
      :class="[
        '-ml-2 pt-2 pb-[3px] pl-2',
        layout === 'grid'
          ? 'grid [grid-template-columns:repeat(auto-fill,minmax(185px,1fr))] items-start gap-4'
          : 'flex items-start gap-[9px] overflow-x-auto [scrollbar-width:none] [&::-webkit-scrollbar]:hidden',
      ]"
    >
      <div
        v-for="c in side.cards"
        :key="tradeCardKey(c)"
        data-part="tile"
        :class="['relative flex flex-none flex-col gap-[5px]', tileSize]"
      >
        <div data-part="face" :class="['relative', dimmed ? 'opacity-[0.82]' : '']">
          <MtgCard
            :name="c.name"
            :image-url="c.image_url"
            :foil="c.foil"
            :qty="c.quantity > 1 ? c.quantity : undefined"
            clickable
            class="shadow-[0_4px_8px_rgba(0,0,0,0.45)]"
            @click="emit('open', c)"
          />
        </div>
        <button
          v-if="removable"
          type="button"
          :class="[
            badge,
            'bg-red-500 text-white hover:bg-red-600 disabled:pointer-events-none disabled:opacity-50',
          ]"
          :aria-label="`Retirer ${c.name}`"
          :disabled="busy"
          @click="emit('remove', c)"
        >
          <Icon name="lucide:minus" :size="13" class="block" />
        </button>
        <span
          v-else-if="reserved"
          :class="[
            badge,
            'text-secondary-ink bg-[color-mix(in_oklch,var(--secondary)_35%,var(--bg))]',
          ]"
          role="img"
          aria-label="Carte réservée"
        >
          <Icon name="lucide:lock" :size="12" class="block" />
        </span>
        <span class="truncate text-[11.5px] text-[var(--ink-2)]">{{ c.name }}</span>
        <span :class="['font-mono text-[12.5px] font-semibold tabular-nums', valueClass]">{{
          formatPrice(tradeCardValue(c))
        }}</span>
      </div>

      <!-- Sans emplacement d'ajout, un côté vide garde sa place avec une tuile fantôme muette. -->
      <div
        v-if="!side.cards.length && !addLabel"
        data-part="ghost"
        role="img"
        :aria-label="emptyMessage"
        :class="placeholder"
      >
        <Icon name="lucide:layers" :size="26" class="opacity-55" />
      </div>

      <button
        v-if="addLabel"
        type="button"
        data-part="add"
        :class="[
          placeholder,
          'hover:border-secondary/50 hover:bg-secondary/10 focus-visible:border-secondary/50 focus-visible:bg-secondary/10 transition-colors duration-150',
        ]"
        @click="emit('add')"
      >
        <Icon name="lucide:search" :size="17" />
        <span class="text-center text-[11px]">{{ addLabel }}</span>
      </button>
    </div>
  </div>
</template>
