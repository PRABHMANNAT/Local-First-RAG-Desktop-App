/**
 * App root. M0 renders a minimal calm shell; real surfaces (workspace rail,
 * sources panel, chat) land in M0's shell task and grow through M1+.
 */
export default function App() {
  return (
    <div className="flex h-full items-center justify-center bg-paper text-ink">
      <div className="text-center">
        <h1 className="font-display text-2xl">Mnemos</h1>
        <p className="mt-2 text-sm text-ink-muted">
          Local-first answers, with receipts.
        </p>
      </div>
    </div>
  );
}
