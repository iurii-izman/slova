export function extractErrorMessage(err: unknown): string {
  if (err instanceof Error && err.message) {
    return err.message;
  }

  if (typeof err === "string") {
    return err;
  }

  if (err && typeof err === "object") {
    const e = err as Record<string, unknown>;

    if (typeof e.message === "string") {
      return e.message;
    }

    if (e.error && typeof e.error === "object") {
      const nested = e.error as Record<string, unknown>;
      if (typeof nested.message === "string") {
        return nested.message;
      }
    }

    try {
      return JSON.stringify(err);
    } catch {
      return "Unknown error";
    }
  }

  return "Unknown error";
}
