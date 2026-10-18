export interface ReportData {
  title: string; subtitle: string; mainLabel: string; main: string; secondary: string;
  categoryTitle: string; topTitle: string; basis: string; legend: string; footer: string;
  categories: {name: string; value: string; fraction: number}[];
  top: {name: string; value: string; fraction: number; actualFraction: number}[];
}
// A dedicated print layout avoids clipping scrollable windows and needs no chart dependency.
export function drawReport(report: ReportData): HTMLCanvasElement {
  const width = 960, height = 600 + (report.categories.length + report.top.length) * 72;
  const canvas = document.createElement("canvas"); canvas.width = width * 2; canvas.height = height * 2;
  const ctx = canvas.getContext("2d"); if (!ctx) throw new Error("Canvas unavailable");
  ctx.scale(2, 2); ctx.fillStyle = "#fff"; ctx.fillRect(0,0,width,height);
  const font = (size: number, weight = 400) => { ctx.font = `${weight} ${size}px -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif`; };
  const text = (value: string, x: number, y: number, size = 20, color = "#222", weight = 400, maxWidth = 840) => {
    font(size, weight); ctx.fillStyle = color;
    let clipped = value; while (ctx.measureText(clipped).width > maxWidth && clipped.length) clipped = clipped.slice(0,-1);
    if (clipped !== value) clipped = clipped.slice(0,-1) + "…";
    ctx.fillText(clipped,x,y);
  };
  const line = (y: number) => { ctx.fillStyle = "#e7e7e9"; ctx.fillRect(60,y,840,1); };
  text("Subbit",60,66,25,"#222",650); text(report.title,60,112,30,"#222",600);
  text(report.subtitle,60,150,17,"#727277");
  text(report.mainLabel,60,208,19,"#727277"); text(report.main,60,272,54,"#222",650);
  text(report.secondary,60,314,19,"#727277"); line(346);
  let y = 392;
  const section = (title: string, rows: {name: string; value: string; fraction: number; actualFraction?: number}[], stacked = false) => {
    text(title,60,y,23,"#222",600); font(17); const label = report.basis; text(label,900-ctx.measureText(label).width,y,17,"#727277"); y += 46;
    for (const row of rows) {
      text(row.name,60,y,20,"#222",500,440); font(18); text(row.value,900-ctx.measureText(row.value).width,y,18,"#555",400,380);
      ctx.fillStyle = "#f1f1f3"; ctx.fillRect(60,y+14,840,10);
      ctx.fillStyle = stacked ? "#c7c7cc" : "#303034"; ctx.fillRect(60,y+14,840*Math.min(1,Math.max(0,row.fraction)),10);
      if (stacked) { ctx.fillStyle = "#303034"; ctx.fillRect(60,y+14,840*Math.min(1,Math.max(0,row.actualFraction ?? 0)),10); }
      y += 72;
    }
  };
  section(report.categoryTitle,report.categories); y += 12; section(report.topTitle,report.top,true);
  text(report.legend,60,y-8,16,"#727277"); line(height-70); text(report.footer,60,height-32,15,"#727277");
  return canvas;
}
