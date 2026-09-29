export default function formatPrice(value?: number) {
  return Intl.NumberFormat('fr-FR', {
    style: 'currency',
    maximumFractionDigits: 2,
    minimumFractionDigits: 2,
    trailingZeroDisplay: 'stripIfInteger',
    currency: 'EUR',
  }).format((value ?? 0) / 100);
}

// Chart values are in euros. Below 100 € the cents matter; above, they only clutter the chart.
const CHART_CENTS_THRESHOLD_EUROS = 100;

export const formatChartPrice = (euros: number) =>
  formatPrice(
    Math.abs(euros) < CHART_CENTS_THRESHOLD_EUROS
      ? Math.round(euros * 100)
      : Math.round(euros) * 100,
  );
