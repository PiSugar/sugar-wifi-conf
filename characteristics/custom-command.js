const execSync = require('child_process').execSync
const exec = require('child_process').exec
let util = require('util')
let bleno = require('@stoprocent/bleno')
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
  if (jsonPath) {
    let result = JSON.parse(fs.readFileSync(jsonPath))
    customArray = result.commands
    console.log('Custom Command Characteristics')
    console.log(customArray)
  }
  customArray.map(function (item, index) {

    let uuidEnd = guid4(index)
    console.log(UUID.CUSTOM_COMMAND_LABEL + uuidEnd)

    item.labelChar = new BlenoCharacteristic({
      uuid: UUID.CUSTOM_COMMAND_LABEL + uuidEnd,
      properties: ['read'],
      value: Buffer.from(item.label),
      descriptors: [
        new BlenoDescriptor({
          uuid: uuidEnd,
          value: 'PiSugar Custom Command ' + item.label
        })
      ]
    })

    item.uuid = UUID.CUSTOM_COMMAND_LABEL + uuidEnd
    characteristicArray.push(item.labelChar)
    return item
  })

  characteristicArray.push(new BlenoCharacteristic({
      uuid: UUID.CUSTOM_COMMAND_COUNT,
      properties: ['read'],
      value: Buffer.from(`${customArray.length}`),
      descriptors: [
        new BlenoDescriptor({
          uuid: UUID.CUSTOM_COMMAND_COUNT,
          value: 'Custom Command Count'
        })
      ]
    }))
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

const InputCharacteristicSep = new BlenoCharacteristic({
  uuid: UUID.CUSTOM_COMMAND_INPUT,
  properties: ['write', 'writeWithoutResponse'],
  onWriteRequest: function(data, offset, withoutResponse, callback) {
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
      for (let i in customArray) {
        if (customArray[i].uuid.toUpperCase() === commandUuid || customArray[i].uuid.toUpperCase().substr(8) === commandUuid) {
          commandToExecute = customArray[i].command
          break;
        }
      }
    }
    callback(this.RESULT_SUCCESS)
    if (isLast) {
      if (commandToExecute) {
        exec(commandToExecute, (error, stdout, stderr) => {
          if (error) {
            response(`exec error: ${error}`)
            return
          }
          response(stdout)
        })
        response('exec done.\n')
      } else {
        response("Command not found.")
      }
    }
  }
})

characteristicArray.push(InputCharacteristicSep)


// NotifyMassage

let message = ''
let messageTimestamp = 0

const NotifyMassageCharacteristic = new BlenoCharacteristic({
  uuid: UUID.CUSTOM_COMMAND_NOTIFY,
  properties: ['notify'],
  onSubscribe: function(maxValueSize, updateValueCallback) {
    console.log('Notify Custom Command subscribe')
    this.timeStamp = messageTimestamp
    this.changeInterval = setInterval(function() {
      if (this.timeStamp === messageTimestamp) return
      let data = Buffer.from(message)
      console.log('Notify Custom Command update value: ' + message)
      updateValueCallback(data)
      this.timeStamp = messageTimestamp
    }.bind(this), 100)
  },
  onUnsubscribe: function() {
    console.log('Notify Custom Command unsubscribe')
  
    if (this.changeInterval) {
      clearInterval(this.changeInterval)
      this.changeInterval = null
    }
  },
  onNotify: function() {
    console.log('Notify Custom Command on notify')
  }
})

characteristicArray.push(NotifyMassageCharacteristic)

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

function guid4(index) {
  let string = (index + 1).toString(16)
  string = '0'.repeat(4 - string.length) + string
  return string
}

module.exports = characteristicArray
