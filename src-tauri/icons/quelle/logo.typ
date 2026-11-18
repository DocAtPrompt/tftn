// TFTN-Logo: das große T, unter seinen Flügeln links ein f, rechts ein n; der Stamm des T ist
// zugleich das t von „ftn“ (T + ftn = TFTN). Schrift: Libertinus Serif (OFL, liegt bei den
// Dokumenttypen). Setzen (aus dem Projektordner):
//   typst compile --ignore-system-fonts --font-path types/tftn-fachartikel/schriften \
//     src-tauri/icons/quelle/logo.typ logo.png --ppi 72
// Daraus die App-Symbole:  npx tauri icon logo.png --output src-tauri/icons
// Eingaben: hintergrund=nein (nur die Buchstaben, durchsichtig), farbe=dunkel (Buchstaben petrol,
// für helle Flächen; nur ohne Hintergrund).
#let groesse = 1024pt
#let grund = rgb(38, 70, 83)
#let hell = rgb(244, 241, 222)
#let mit-hintergrund = sys.inputs.at("hintergrund", default: "ja") == "ja"
#let tinte = if not mit-hintergrund and sys.inputs.at("farbe", default: "hell") == "dunkel" { grund } else { hell }
#set page(width: groesse, height: groesse, margin: 0pt, fill: none)
#set text(font: "Libertinus Serif", fill: tinte)
#if mit-hintergrund {
  place(center + horizon, rect(width: 82%, height: 82%, radius: 18%, fill: grund))
}
// Grundlinie gemeinsam; das T steht mittig. Das n hält unten 89 pt Abstand zum Stamm, das f ist
// größer gesetzt und reicht höher unter den Flügel; sein Kopf endet 49 pt vor dem Stamm.
#let gl = 727.358pt
#let zeichen(x, groesse, z) = place(top + left, dx: x, dy: gl, place(bottom + center, text(size: groesse, z)))
#zeichen(512pt, 660pt, "T")
#zeichen(345pt, 400pt, "f")
#zeichen(709pt, 300pt, "n")
