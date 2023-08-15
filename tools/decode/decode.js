// Original: https://www.bugsnag.com/blog/source-maps/
//
// Used exclusively for diagnostics and not included
// in the distribution.
//
//      cat my-source-map.js.map | node decode | jq .
//

const fs = require("fs")
const concat = require('concat-stream')
const vlq = require('vlq')

const formatMappings = (mappings, sources, names) => {
  var vlqState = [ 0, 0, 0, 0, 0 ]
  return mappings.split(';').reduce((accum, line, i) => {
    accum[i + 1] = formatLine(line, vlqState, sources, names)
    vlqState[0] = 0
    return accum
  }, {})
}

const formatLine = (line, state, sources, names) => {
  const segs = line.split(',')
  return segs.map(seg => {
    if (!seg) return ''
    const decoded = vlq.decode(seg)
    for (var i = 0; i < 5; i++) {
      state[i] = typeof decoded[i] === 'number' ? state[i] + decoded[i] : state[i]
    }
    return formatSegment(...state.concat([ sources, names ]))
  })
}

const formatSegment = (col, source, sourceLine, sourceCol, name, sources, names) =>
  `${col + 1} => ${sources[source]} ${sourceLine + 1}:${sourceCol + 1}${names[name] ? ` ${names[name]}` : ``}`

const sourcemap = process.argv[process.argv.length - 2]
const output = process.argv[process.argv.length - 1]
const json = fs.readFileSync(sourcemap)
const map = JSON.parse(json)
fs.writeFileSync(output, JSON.stringify({
  ...map,
  mappings: formatMappings(map.mappings, map.sources, map.names)
}))
