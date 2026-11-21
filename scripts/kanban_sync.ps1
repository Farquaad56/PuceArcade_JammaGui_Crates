# Met à jour le board Hermes après une révision des cartes (découpage, nouveaux textes) SANS dupliquer :
# - create avec la même clé d'idempotence renvoie l'id existant (ou crée les nouvelles cartes, ex. T203b) ;
# - edit remplace titre et corps des cartes à partir de -From (les cartes antérieures, terminées ou en cours, ne sont pas modifiées) ;
# - link ajoute les dépendances manquantes (une dépendance déjà présente est ignorée).
param([string]$Root = "H:\__Emulator__\PuceArcade_JammaGui_Crates", [string]$Assignee = "rust-dev", [string]$BoardSlug = "jamma",
      [string]$KeyPrefix = "pucearcade-jamma", [string]$From = "J000")
$ErrorActionPreference = "Continue"
& hermes kanban boards switch $BoardSlug | Out-Null
$cards = Get-Content "$Root\tasks\board.json" -Raw -Encoding UTF8 | ConvertFrom-Json
$ids = @{}; $active = $false
foreach ($t in $cards) {
  if ($t.id -eq $From) { $active = $true }
  $a = @("kanban","create",$t.title,"--body-file","$Root\$($t.file)","--idempotency-key","$KeyPrefix-$($t.id)","--workspace","dir:$Root","--tenant",$BoardSlug,"--assignee",$Assignee,"--json")
  foreach ($d in $t.deps) { if ($ids.ContainsKey($d)) { $a += @("--parent",$ids[$d]) } }
  $raw = & hermes @a
  try { $out = $raw | ConvertFrom-Json } catch { Write-Host "ERREUR $($t.id) : $raw"; exit 1 }
  $id = if ($out.task_id) { $out.task_id } else { $out.id }
  if (-not $id) { Write-Host "ERREUR $($t.id) : pas d'id dans $raw"; exit 1 }
  $ids[$t.id] = $id
  if (-not $active -or $t.status -eq "done") { continue }
  $body = Get-Content "$Root\$($t.file)" -Raw -Encoding UTF8
  & hermes kanban edit $id --title $t.title --body $body | Out-Null
  foreach ($d in $t.deps) { if ($ids.ContainsKey($d)) { & hermes kanban link $ids[$d] $id 2>&1 | Out-Null } }
  Write-Host "$($t.id) -> $id (mis à jour)"
}
Write-Host "Synchronisation terminée."
