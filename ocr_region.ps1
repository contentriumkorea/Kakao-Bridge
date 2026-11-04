function Get-KakaoListCrop([int[]]$root,[int[]]$child,[int]$width,[int]$height) {
  if($root.Count -ne 4 -or $child.Count -ne 4 -or $width -le 0 -or $height -le 0){throw 'LIST_GEOMETRY_UNAVAILABLE'}
  if(($root[2]-$root[0]) -ne $width -or ($root[3]-$root[1]) -ne $height){throw 'LIST_GEOMETRY_UNAVAILABLE'}
  $x=$child[0]-$root[0];$y=$child[1]-$root[1];$w=$child[2]-$child[0];$h=$child[3]-$child[1]
  if($x -lt 0 -or $y -lt 0 -or $w -le 0 -or $h -le 0 -or ($x+$w) -gt $width -or ($y+$h) -gt $height){throw 'LIST_GEOMETRY_UNAVAILABLE'}
  return @{x=$x;y=$y;width=$w;height=$h}
}
