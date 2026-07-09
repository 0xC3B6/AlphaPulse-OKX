import type { Copy } from "./i18n";

export function PaginationControls({
  className = "",
  copy,
  onPageChange,
  page,
  pageCount,
  testId,
  total,
}: {
  className?: string;
  copy: Copy;
  onPageChange: (page: number) => void;
  page: number;
  pageCount: number;
  testId: string;
  total: number;
}) {
  const safePageCount = Math.max(1, pageCount);
  const safePage = Math.min(Math.max(page, 0), safePageCount - 1);

  return (
    <nav className={`pagination-controls ${className}`} data-testid={testId}>
      <span>
        {formatPaginationTemplate(copy.pagination.pageStatus, {
          page: safePage + 1,
          pageCount: safePageCount,
          total,
        })}
      </span>
      <span className="pagination-total">
        {formatPaginationTemplate(copy.pagination.total, {
          page: safePage + 1,
          pageCount: safePageCount,
          total,
        })}
      </span>
      <button
        disabled={safePage <= 0}
        onClick={() => onPageChange(safePage - 1)}
        type="button"
      >
        {copy.pagination.previous}
      </button>
      <button
        disabled={safePage >= safePageCount - 1}
        onClick={() => onPageChange(safePage + 1)}
        type="button"
      >
        {copy.pagination.next}
      </button>
    </nav>
  );
}

function formatPaginationTemplate(
  template: string,
  values: { page: number; pageCount: number; total: number },
) {
  return template
    .replace("{page}", String(values.page))
    .replace("{pageCount}", String(values.pageCount))
    .replace("{total}", String(values.total));
}
