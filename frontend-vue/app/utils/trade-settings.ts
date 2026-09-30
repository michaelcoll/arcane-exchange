/* Règles de mise à l'échange (écran Profil).
 * Les binders ManaBox et les règles par rareté viennent du backend
 * (/collection/stats, /user/trade-binders, /collection/visibility/rarities). */

export const MAX_KEPT_COPIES = 4;

export const fmtInt = (n: number): string => Math.round(n).toLocaleString('fr-FR');
