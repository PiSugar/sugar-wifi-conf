const execSync = require('child_process').execSync
let util = require('util')
let bleno = require('bleno')
let UUID = require('../sugar-uuid')

let BlenoCharacteristic = bleno.Characteristic

let config = {
  key: 'pisugar'
}
// Input

let InputCharacteristic = function() {
  InputCharacteristic.super_.call(this, {
    uuid: UUID.INPUT,
    properties: ['write', 'writeWithoutResponse']
  })
}

util.inherits(InputCharacteristic, BlenoCharacteristic)

InputCharacteristic.prototype.onWriteRequest = function(data, offset, withoutResponse, callback) {
  console.log('InputCharacteristic write request: ' + data.toString() + ' ' + offset + ' ' + withoutResponse)
  let inputArray = data.toString().split('%&%')
  if (inputArray.length !== 3) {
    console.log('Wrong input syntax')
    return
  }
  if (inputArray[0] !== config.key){
    console.log('Wrong input key')
    return
  }
  let ssid = inputArray[1]
  let password = inputArray[2]
  let result = setWifi(ssid, password)
  callback(this.RESULT_SUCCESS)
}

// NotifyMassage

let NotifyMassageCharacteristic = function() {
  NotifyMassageCharacteristic.super_.call(this, {
    uuid: UUID.NOTIFY_MESSAGE,
    properties: ['notify']
  })
}

util.inherits(NotifyMassageCharacteristic, BlenoCharacteristic)

NotifyMassageCharacteristic.prototype.onSubscribe = function(maxValueSize, updateValueCallback) {
  console.log('NotifyMassageCharacteristic subscribe')

  this.counter = 0
  this.changeInterval = setInterval(function() {
    let data = new Buffer(4)
    data.writeUInt32LE(this.counter, 0)

    console.log('NotifyMassageCharacteristic update value: ' + this.counter)
    updateValueCallback(data)
    this.counter++
  }.bind(this), 5000)
}

NotifyMassageCharacteristic.prototype.onUnsubscribe = function() {
  console.log('NotifyMassageCharacteristic unsubscribe')

  if (this.changeInterval) {
    clearInterval(this.changeInterval)
    this.changeInterval = null
  }
}

NotifyMassageCharacteristic.prototype.onNotify = function() {
  console.log('NotifyMassageCharacteristic on notify')
}

function setWifi(ssid, password) {
  let error = 'ok'
  try {
    execSync(`nmcli device wifi con "${ssid}" password "${password}"`)
  } catch (e) {
    error = e.toString()
    console.log(error)
  }
  return error
}

module.exports = {
  InputCharacteristic,
  NotifyMassageCharacteristic
}