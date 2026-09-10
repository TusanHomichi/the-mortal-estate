/** Paint decoded, verified pixels once. The packet's temporary blob URLs are retired. */
export function prepareEntryBackdrop(image: HTMLImageElement): void {
  const canvas = document.createElement("canvas");
  canvas.id = "entry-scene"; canvas.setAttribute("aria-hidden", "true");
  canvas.width = image.naturalWidth; canvas.height = image.naturalHeight;
  const context = canvas.getContext("2d");
  if (!context) throw new Error("Entry artwork could not be prepared.");
  context.drawImage(image, 0, 0);
  document.getElementById("entry")!.prepend(canvas);
  const resize = () => {
    const scale = Math.max(1, Math.ceil(Math.max(innerWidth / canvas.width, innerHeight / canvas.height)));
    canvas.style.width = `${canvas.width * scale}px`; canvas.style.height = `${canvas.height * scale}px`;
  };
  resize(); window.addEventListener("resize", resize);
}
