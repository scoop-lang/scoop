// Optional external-tool recipe. The Scoop library communicates only through stdin/stdout.
import { pathToFileURL } from 'node:url';
import { resolve } from 'node:path';
import { readFileSync } from 'node:fs';

const root = process.argv[2];
if (!root) throw new Error('Usage: node render.mjs <installed @mathjax/src directory>');
const { init } = await import(pathToFileURL(resolve(root, 'bundle/node-main.mjs')).href);
const input = JSON.parse(readFileSync(0, 'utf8'));
const MathJax = await init({
  loader: { load: ['input/tex', 'output/svg'] },
  svg: { fontCache: 'none', displayOverflow: 'overflow', linebreaks: { inline: false } },
  startup: { typeset: false },
});
const adaptor = MathJax.startup.adaptor;
const em = input.fontSize;
const ex = em * MathJax.startup.document.outputJax.font.params.x_height;
const node = await MathJax.tex2svgPromise(input.source, {
  display: !input.inline, em, ex, containerWidth: input.availableWidth,
});
const svg = adaptor.tags(node, 'svg')[0];
if (!svg) throw new Error('MathJax did not return SVG');
const box = adaptor.getAttribute(svg, 'viewBox');
const bounds = box || adaptor.getAttribute(svg, 'data-mjx-viewBox');
if (!bounds) throw new Error('MathJax did not return SVG bounds');
const [x, y, w, h] = bounds.split(/\s+/).map(Number);
// MathJax SVG.createSVG uses 1000 drawing units per em. The baseline is y=0;
// -viewBox.y is its actual distance from the top, including ascenders and depth.
let width = w * em / 1000;
const height = h * em / 1000;
const baseline = -y * em / 1000;
// Full-width tagged equations keep their bounds as data-mjx-viewBox and already
// transform drawing units into CSS pixels. Preserve that transform and the nested
// percentage layout, while giving the rasterizer a definite outer viewport.
if (!box) {
  const relativeWidth = adaptor.getAttribute(svg, 'width');
  if (!/^\d+(?:\.\d+)?%$/.test(relativeWidth)) {
    throw new Error('MathJax returned an invalid relative SVG width');
  }
  width = Math.max(width, input.availableWidth * Number.parseFloat(relativeWidth) / 100);
}
if (![x, y, width, height, baseline].every(Number.isFinite) || width <= 0 || height <= 0) {
  throw new Error('MathJax returned invalid SVG metrics');
}
adaptor.setAttribute(svg, 'width', `${width}px`);
adaptor.setAttribute(svg, 'height', `${height}px`);
adaptor.setAttribute(svg, 'color', input.foreground);
process.stdout.write(JSON.stringify({ width, height, baseline, svg: adaptor.outerHTML(svg) }));
