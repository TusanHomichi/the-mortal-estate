/**
 * Camera-space wall selection for the browser feel scene's walk experiment.
 *
 * This is presentation math only. It decides which authored wall runs may hide
 * the locally presented caretaker; it decides neither sight nor walkability.
 */
import type { WallRun } from "../feelTypes";
import type { Cell } from "./layoutPassability";

export function occludingRuns(
  runs: readonly WallRun[],
  playerCell: Cell,
  coverTiles: number,
): WallRun[] {
  if (!Number.isFinite(coverTiles) || coverTiles < 0) return [];

  return runs.filter((run) => {
    const [startI, startJ] = run.start;
    if (run.axis === "x") {
      const depthBehind = startJ - playerCell.j;
      return (
        depthBehind > 0 &&
        depthBehind <= coverTiles &&
        playerCell.i >= startI - 1 &&
        playerCell.i <= startI + run.cells + 1
      );
    }

    const depthBehind = startI - playerCell.i;
    return (
      depthBehind > 0 &&
      depthBehind <= coverTiles &&
      playerCell.j >= startJ - 1 &&
      playerCell.j <= startJ + run.cells + 1
    );
  });
}
