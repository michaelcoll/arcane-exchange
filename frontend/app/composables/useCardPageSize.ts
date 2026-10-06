export type CardSize = 'sm' | 'md' | 'lg';

const PAGE_SIZE_BY_CARD_SIZE: Record<CardSize, number> = {
  sm: 40,
  md: 20,
  lg: 10,
};

const MOBILE_PAGE_SIZE = 10;

export const useCardPageSize = (
  size: MaybeRefOrGetter<CardSize>,
  isDesktop: MaybeRefOrGetter<boolean>,
) =>
  computed(() => (toValue(isDesktop) ? PAGE_SIZE_BY_CARD_SIZE[toValue(size)] : MOBILE_PAGE_SIZE));
