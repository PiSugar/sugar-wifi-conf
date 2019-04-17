const execSync = require('child_process').execSync
let util = require('util')
let bleno = require('bleno')
let UUID = require('../sugar-uuid')
let config = require('../config')
const fs = require('fs')
const concatTag = '%&%'
const endTag = '&#&'

let jsonPath
let characteristicArray = []
let customArray = []

let BlenoCharacteristic = bleno.Characteristic
let BlenoDescriptor = bleno.Descriptor

let argv = process.argv
if (argv.length > 2) config.key = process.argv[2]
if (argv.length > 3) jsonPath = process.argv[3]


try {
  let result = JSON.parse(fs.readFileSync(jsonPath))
  customArray = result.commands
  console.log('Custom Command Characteristics')
  console.log(customArray)
  customArray.map(function (item) {

    let uuidEnd = guid4()

    let labelCharacteristic = function() {
      labelCharacteristic.super_.call(this, {
        uuid: UUID.CUSTOM_COMMAND_LABEL + uuidEnd,
        properties: ['read'],
        value: new Buffer(item.label),
        descriptors: [
          new BlenoDescriptor({
            uuid: uuidEnd,
            value: 'PiSugar Custom Command ' + item.label
          })
        ]
      })
    }
    console.log('func created')
    util.inherits(labelCharacteristic, BlenoCharacteristic)
    console.log('func created 2')
    item.labelChar = new labelCharacteristic()
    console.log('func created 3')
    item.uuid = UUID.CUSTOM_COMMAND_LABEL + uuidEnd
    console.log('func created 4')
    characteristicArray.push(item.labelChar)
    console.log('func created 5')
    return item
  })
} catch (e) {
  console.log(e)
}

// Input android

let separateInputString = ''
let separateInputStringCopy = ''
let lastChangeTime = 0
let clearTime = 5000

setInterval(function () {
  if (separateInputStringCopy !== separateInputString) {
    separateInputStringCopy = separateInputString
    lastChangeTime = new Date().getTime()
  } else if (new Date().getTime() - lastChangeTime > clearTime && separateInputString !== '') {
    lastChangeTime = new Date().getTime()
    separateInputStringCopy = ''
    separateInputString = ''
    console.log('clear separateInputString')
  }
}, 1000)

let InputCharacteristicSep = function() {
  InputCharacteristicSep.super_.call(this, {
    uuid: UUID.CUSTOM_COMMAND_INPUT,
    properties: ['write', 'writeWithoutResponse']
  })
}

util.inherits(InputCharacteristicSep, BlenoCharacteristic)

InputCharacteristicSep.prototype.onWriteRequest = function(data, offset, withoutResponse, callback) {
  console.log('InputCharacteristicSep write request: ' + data.toString() + ' ' + offset + ' ' + withoutResponse)
  separateInputString += data.toString()
  let isLast = separateInputString.indexOf(endTag) >= 0
  let commandToExecute
  let commandUuid
  if (isLast) {
    separateInputString = separateInputString.replace(endTag, '')
    let inputArray = separateInputString.split(concatTag)
    lastChangeTime = new Date().getTime()
    separateInputStringCopy = ''
    separateInputString = ''
    if (inputArray && inputArray.length < 2) {
      console.log('Invalid syntax.')
      setMessage('Invalid syntax.')
      callback(this.RESULT_SUCCESS)
      return
    }
    if (inputArray[0] !== config.key) {
      console.log('Invalid key.')
      setMessage('Invalid key.')
      callback(this.RESULT_SUCCESS)
      return
    }
    try {
      commandUuid = inputArray[1].split('-').splice(-1)[0].toUpperCase()
    } catch (e) {
      console.log('Invalid UUID.')
      setMessage('Invalid UUID.')
      callback(this.RESULT_SUCCESS)
      return
    }
  }
  callback(this.RESULT_SUCCESS)
  for (let i in customArray) {
    if (customArray[i].uuid.toUpperCase() === commandUuid) {
      commandToExecute = customArray[i].command
      break;
    }
  }
  if (commandToExecute) {
    response(exec(commandToExecute))
  } else {
    response("Command not found.")
  }
}

characteristicArray.push(new InputCharacteristicSep())


// NotifyMassage

let message = ''
let messageTimestamp = 0

let NotifyMassageCharacteristic = function() {
  NotifyMassageCharacteristic.super_.call(this, {
    uuid: UUID.CUSTOM_COMMAND_NOTIFY,
    properties: ['notify']
  })
}

util.inherits(NotifyMassageCharacteristic, BlenoCharacteristic)

NotifyMassageCharacteristic.prototype.onSubscribe = function(maxValueSize, updateValueCallback) {
  console.log('Notify Custom Command subscribe')
  this.timeStamp = messageTimestamp
  this.changeInterval = setInterval(function() {
    if (this.timeStamp === messageTimestamp) return
    let data = new Buffer(message)
    console.log('Notify Custom Command update value: ' + message)
    updateValueCallback(data)
    this.timeStamp = messageTimestamp
  }.bind(this), 100)
}

NotifyMassageCharacteristic.prototype.onUnsubscribe = function() {
  console.log('Notify Custom Command unsubscribe')

  if (this.changeInterval) {
    clearInterval(this.changeInterval)
    this.changeInterval = null
  }
}

NotifyMassageCharacteristic.prototype.onNotify = function() {
  console.log('Notify Custom Command on notify')
}

characteristicArray.push(new NotifyMassageCharacteristic())

function exec (cmd) {
  try {
    let value = execSync(cmd).toString().trim()
    if (value === '') value = 'success'
    return value
  } catch (e) {
    return e.toString()
  }
}

async function response (string) {
  message = ''
  string += endTag
  let msgArray = []
  for (let i in string) {
    if (i % 20 == 0) {
      msgArray.push('')
    }
    msgArray[msgArray.length - 1] += string[i]
  }
  for (let i in msgArray) {
    setMessage(msgArray[i].toString())
    await sleep(200)
  }
}

function sleep (time) {
  return new Promise(function (resolve) {
    setTimeout(function () {
      resolve(1)
    }, time)
  })
}


function setMessage (msg) {
  message = msg
  messageTimestamp = new Date().getTime()
}

function guid4 () {
  return 'xxxx'.replace(/[xy]/g, function(c) {
    let r = Math.random()*16|0, v = c === 'x' ? r : (r&0x3|0x8)
    return v.toString(16)
  })
}

module.exports = characteristicArray
