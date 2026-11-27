// Generates the Microsoft Store tile images from the brand app icon. Run once after the icon changes:
//   node scripts/make-tiles.mjs
import { mkdirSync } from 'node:fs'
import sharp from 'sharp'

const icon = 'assets/brand/icons/app-icon-dark-1024.png'
const ground = { r: 17, g: 17, b: 20, alpha: 1 } // surface-1, Ice Dark (#111114)
mkdirSync('build/appx', { recursive: true })

const square = (size, name) => sharp(icon).resize(size, size).png().toFile(`build/appx/${name}.png`)
// Wide tile: the icon centred on the brand's dark surface.
const wide = async (w, h, name) => {
  const mark = await sharp(icon).resize(Math.round(h * 0.7), Math.round(h * 0.7)).png().toBuffer()
  await sharp({ create: { width: w, height: h, channels: 4, background: ground } })
    .composite([{ input: mark, gravity: 'center' }])
    .png()
    .toFile(`build/appx/${name}.png`)
}

await Promise.all([
  square(50, 'StoreLogo'),
  square(44, 'Square44x44Logo'),
  square(150, 'Square150x150Logo'),
  wide(310, 150, 'Wide310x150Logo'),
])
console.log('tiles written to build/appx')
