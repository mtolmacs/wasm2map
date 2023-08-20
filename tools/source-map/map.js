const fs = require("fs")
const {
    SourceMapConsumer,
} = require("source-map")

const json = fs.readFileSync(process.argv[process.argv.length - 1]).toString()
const map = SourceMapConsumer.with(json, null, consumer => {
    console.log(
        consumer.originalPositionFor({
            line: 1,
            column: 7709,
        })
    )
})
