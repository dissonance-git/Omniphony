// Headphone L/R output meters for the binaural mode.
//
// The binaural stage's stereo output is broadcast through the first two
// speaker meter slots (/omniphony/meter/speaker/0|1); speakers.js routes
// those here. Bars show RMS with a thin peak marker, on the usual
// -60..0 dBFS scale, displayed in the renderer panel's Output block (CSS
// shows them only while body.output-binaural is set).

const FLOOR_DB = -60;

function pct(db) {
  const v = Number.isFinite(db) ? db : -100;
  return Math.max(0, Math.min(100, ((v - FLOOR_DB) / -FLOOR_DB) * 100));
}

export function updateHeadphoneMeter(index, level) {
  const side = index === 0 ? 'L' : 'R';
  const fill = document.getElementById(`hpMeterFill${side}`);
  const peak = document.getElementById(`hpMeterPeak${side}`);
  if (!fill) return;
  fill.style.width = `${pct(level?.rmsDbfs).toFixed(1)}%`;
  if (peak) peak.style.left = `${pct(level?.peakDbfs).toFixed(1)}%`;
}
