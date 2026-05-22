# Generates SK tray icons for the holo and memphis theme families.
# Run with:  pwsh app/src-tauri/icons/generate-sk.ps1
#
# Output:
#   sk-holo.png     — rounded gradient capsule with "SK"
#   sk-memphis.png  — hard-edge square with "SK"

Add-Type -AssemblyName System.Drawing

$here = Split-Path -Parent $MyInvocation.MyCommand.Path

function New-SkIcon {
    param(
        [string]$OutPath,
        [string]$FontName,
        [System.Drawing.Color]$BgFrom,
        [System.Drawing.Color]$BgTo,
        [System.Drawing.Color]$TextColor,
        [bool]$Rounded
    )

    $size = 256
    $bmp = New-Object System.Drawing.Bitmap $size, $size
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
    $g.TextRenderingHint = [System.Drawing.Text.TextRenderingHint]::AntiAliasGridFit
    $g.Clear([System.Drawing.Color]::Transparent)

    $pad = 18
    $rect = New-Object System.Drawing.Rectangle $pad, $pad, ($size - $pad * 2), ($size - $pad * 2)

    if ($Rounded) {
        $gp = New-Object System.Drawing.Drawing2D.GraphicsPath
        $r = 56
        $x = $rect.X; $y = $rect.Y; $w = $rect.Width; $h = $rect.Height
        $gp.AddArc($x, $y, $r, $r, 180, 90)
        $gp.AddArc(($x + $w - $r), $y, $r, $r, 270, 90)
        $gp.AddArc(($x + $w - $r), ($y + $h - $r), $r, $r, 0, 90)
        $gp.AddArc($x, ($y + $h - $r), $r, $r, 90, 90)
        $gp.CloseFigure()

        $p1 = New-Object System.Drawing.PointF $rect.X, $rect.Y
        $p2 = New-Object System.Drawing.PointF ($rect.X + $rect.Width), ($rect.Y + $rect.Height)
        $gradBrush = New-Object System.Drawing.Drawing2D.LinearGradientBrush $p1, $p2, $BgFrom, $BgTo
        $g.FillPath($gradBrush, $gp)
        $gradBrush.Dispose()
        $gp.Dispose()
    } else {
        $bgBrush = New-Object System.Drawing.SolidBrush $BgFrom
        $g.FillRectangle($bgBrush, $rect)
        $bgBrush.Dispose()
        $borderPen = New-Object System.Drawing.Pen $TextColor, 8
        $g.DrawRectangle($borderPen, $rect)
        $borderPen.Dispose()
    }

    # Pick the best installed font; fall back if not present.
    $fontFamilyName = $FontName
    try {
        $testFont = New-Object System.Drawing.FontFamily $fontFamilyName
        $testFont.Dispose()
    } catch {
        $fontFamilyName = if ($Rounded) { 'Segoe UI Black' } else { 'Impact' }
    }

    $fontSize = if ($Rounded) { 110 } else { 130 }
    $bold = [System.Drawing.FontStyle]::Bold
    $unitPixel = [System.Drawing.GraphicsUnit]::Pixel
    $font = New-Object System.Drawing.Font $fontFamilyName, $fontSize, $bold, $unitPixel
    $textBrush = New-Object System.Drawing.SolidBrush $TextColor

    $format = New-Object System.Drawing.StringFormat
    $format.Alignment = [System.Drawing.StringAlignment]::Center
    $format.LineAlignment = [System.Drawing.StringAlignment]::Center

    $textRect = New-Object System.Drawing.RectangleF $rect.X, ($rect.Y - 8), $rect.Width, $rect.Height
    $g.DrawString('SK', $font, $textBrush, $textRect, $format)

    $font.Dispose()
    $textBrush.Dispose()
    $format.Dispose()
    $g.Dispose()

    $bmp.Save($OutPath, [System.Drawing.Imaging.ImageFormat]::Png)
    $bmp.Dispose()
    Write-Host "wrote $OutPath"
}

# Holographic: pink → cyan gradient, rounded, white text
New-SkIcon `
    -OutPath (Join-Path $here 'sk-holo.png') `
    -FontName 'Segoe UI Black' `
    -BgFrom ([System.Drawing.Color]::FromArgb(255, 255, 45, 149)) `
    -BgTo ([System.Drawing.Color]::FromArgb(255, 0, 229, 255)) `
    -TextColor ([System.Drawing.Color]::White) `
    -Rounded $true

# Memphis: hot yellow square, hard edge, black text
New-SkIcon `
    -OutPath (Join-Path $here 'sk-memphis.png') `
    -FontName 'Impact' `
    -BgFrom ([System.Drawing.Color]::FromArgb(255, 255, 217, 61)) `
    -BgTo ([System.Drawing.Color]::FromArgb(255, 255, 217, 61)) `
    -TextColor ([System.Drawing.Color]::Black) `
    -Rounded $false

# Mr Robotic: amber phosphor on black square, terminal vibe
New-SkIcon `
    -OutPath (Join-Path $here 'sk-robotic.png') `
    -FontName 'Consolas' `
    -BgFrom ([System.Drawing.Color]::FromArgb(255, 10, 10, 10)) `
    -BgTo ([System.Drawing.Color]::FromArgb(255, 10, 10, 10)) `
    -TextColor ([System.Drawing.Color]::FromArgb(255, 255, 176, 0)) `
    -Rounded $false

# Corporate Overlord: jet on bone, condensed gothic, brutalist
New-SkIcon `
    -OutPath (Join-Path $here 'sk-corporate.png') `
    -FontName 'Bahnschrift Condensed' `
    -BgFrom ([System.Drawing.Color]::FromArgb(255, 244, 244, 245)) `
    -BgTo ([System.Drawing.Color]::FromArgb(255, 244, 244, 245)) `
    -TextColor ([System.Drawing.Color]::FromArgb(255, 9, 9, 11)) `
    -Rounded $false

Write-Host 'done.'
