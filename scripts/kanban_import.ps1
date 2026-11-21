# Importe tasks/board.json dans Hermes Kanban (board "jamma", profil rust-dev).
# Chaîne linéaire : chaque carte a pour parent(s) ses dépendances. Idempotent (clé pucearcade-jamma-<id>).
param([string]$Root = "H:\__Emulator__\PuceArcade_JammaGui_Crates", [string]$Assignee = "rust-dev", [string]$BoardSlug = "jamma")
$ErrorActionPreference = "Continue"
& hermes kanban boards create $BoardSlug --name "Jamma GUI" 2>&1 | Out-Null
& hermes kanban boards switch $BoardSlug
$cards = Get-Content "$Root\tasks\board.json" -Raw -Encoding UTF8 | ConvertFrom-Json
$ids = @{}
foreach ($t in $cards) {
  if ($t.status -eq "done") { continue }
  $a = @("kanban","create",$t.title,"--body-file","$Root\$($t.file)","--idempotency-key","pucearcade-jamma-$($t.id)","--workspace","dir:$Root","--tenant",$BoardSlug,"--assignee",$Assignee,"--json")
  foreach ($d in $t.deps) { if ($ids.ContainsKey($d)) { $a += @("--parent",$ids[$d]) } }
  $raw = & hermes @a
  try { $out = $raw | ConvertFrom-Json } catch { Write-Host "ERREUR $($t.id) : $raw"; exit 1 }
  $id = if ($out.task_id) { $out.task_id } else { $out.id }
  if (-not $id) { Write-Host "ERREUR $($t.id) : pas d'id dans $raw"; exit 1 }
  $ids[$t.id] = $id
  Write-Host "$($t.id) -> $id"
}
Write-Host "Termine : $($ids.Count) cartes."
