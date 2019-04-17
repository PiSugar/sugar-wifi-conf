const execSync = require('child_process').execSync
let util = require('util')
let bleno = require('bleno')
let UUID = require('../sugar-uuid')
const fs = require('fs')

let jsonPath
let characteristicArray = []
let customArray = []

let BlenoCharacteristic = bleno.Characteristic
let BlenoDescriptor = bleno.Descriptor


let argv = process.argv
if (argv.length > 3) jsonPath = process.argv[3]


try {
  let result = JSON.parse(fs.readFileSync(jsonPath))
  customArray = result.items || result.info
  console.log('Custom Info Characteristics')
  console.log(customArray)
  customArray.map(function (item) {

    let uuidEnd = guid4()

    let labelCharacteristic = function() {
      labelCharacteristic.super_.call(this, {
        uuid: UUID.CUSTOM_INFO_LABEL + uuidEnd,
        properties: ['read'],
        value: new Buffer(item.label),
        descriptors: [
          new BlenoDescriptor({
            uuid: uuidEnd,
            value: 'PiSugar Custom Info ' + item.label
          })
        ]
      })
    }
    util.inherits(labelCharacteristic, BlenoCharacteristic)
    item.labelChar = new labelCharacteristic()

    let valueCharacteristic = function() {
      valueCharacteristic.super_.call(this, {
        uuid: UUID.CUSTOM_INFO + uuidEnd,
        properties: ['notify']
      })
    }

    util.inherits(valueCharacteristic, BlenoCharacteristic)

    valueCharacteristic.prototype.onSubscribe = function(maxValueSize, updateValueCallback) {
      console.log('Custom info subscribe')
      this.getValue = function () {
        try{
          let value = execSync(item.command)
          return value
        } catch (e) {
          console.log(e.toString())
          return 'cmd error'
        }
      }
      this.value = this.getValue()
      updateValueCallback(new Buffer(this.value))
      this.changeInterval = setInterval(function() {
        let newValue = this.getValue()
        if (newValue === this.value) return
        this.value = newValue
        let data = new Buffer(this.value)
        updateValueCallback(data)
      }.bind(this), item.interval ? item.interval * 1000 : 10000)
    }

    valueCharacteristic.prototype.onUnsubscribe = function() {
      console.log('Custom info unsubscribe')

      if (this.changeInterval) {
        clearInterval(this.changeInterval)
        this.changeInterval = null
      }
    }
    valueCharacteristic.prototype.onNotify = function() {
      // console.log('Custom info on notify')
    }
    item.valueChar = new valueCharacteristic()

    characteristicArray.push(item.labelChar)
    characteristicArray.push(item.valueChar)
    return item
  })
} catch (e) {
  console.log(e.toString())
}

function guid4() {
  return 'xxxx'.replace(/[xy]/g, function(c) {
    var r = Math.random()*16|0, v = c == 'x' ? r : (r&0x3|0x8)
    return v.toString(16)
  })
}

module.exports = characteristicArray
