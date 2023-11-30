const fs = require("fs")
const {
    SourceMapConsumer,
} = require("source-map")

const json = fs.readFileSync(process.argv[process.argv.length - 2]).toString()
const map = SourceMapConsumer.with(json, null, consumer => {
    console.log(
        JSON.stringify(
            consumer.originalPositionFor({
                line: 1,
                column: process.argv[process.argv.length - 1],
            })
        )
    )
})
