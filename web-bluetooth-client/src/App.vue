<template>
  <div id="app">
    <img class="logo" src="./assets/logo.svg">
    <div class="panel-container">
      <div v-if="!supportBluetooth" class="connect-panel panel">
        <div class="panel-title">Cannot find web bluetooth.</div>
        <p>Please make sure your device and browser support web bluetooth. Please visit <a href="https://github.com/WebBluetoothCG/web-bluetooth/blob/master/implementation-status.md" target="_blank">Here</a> to check web-bluetooth compatibility.</p>
      </div>
      <div v-if="!isConnected && supportBluetooth" class="connect-panel panel">
        <div class="panel-title">Connect your pi ...</div>
        <el-row>
          <el-button @click="connectDevice">Discover</el-button>
        </el-row>
      </div>
      <div v-if="isConnected" class="info-panel panel">
        <div class="panel-title">Device Information</div>
        <div v-for="item in infoList" :key="item.uuid" class="info-group">
          <div class="label">{{item.label}}</div>
          <div class="value">{{item.value}}</div>
        </div>
      </div>
      <div v-if="isConnected" class="wifi-panel panel">
        <div class="panel-title">Wifi Setting</div>
        <el-row>
          <el-input placeholder="Wifi name" v-model="ssid">
            <template slot="prepend">Wifi</template>
          </el-input>
        </el-row>
        <el-row>
          <el-input placeholder="Wifi password" v-model="password" type="password">
            <template slot="prepend">Psw</template>
          </el-input>
        </el-row>
        <el-row>
          <el-input placeholder="Key (Default: pisugar)" v-model="key">
            <template slot="prepend">Key</template>
          </el-input>
        </el-row>
        <el-row>
          <el-button @click="inputWifi">Confirm</el-button>
        </el-row>
      </div>
      <div v-if="isConnected" class="command-panel panel">
        <div class="panel-title">Custom Commands</div>
        <el-row>
          <div v-for="item in commandList" :key="item.uuid" class="button-wrap"><el-button size="small" round>{{item.label}}</el-button></div>
        </el-row>
        <el-row>
          <el-input placeholder="Key (Default: pisugar)" v-model="key">
            <template slot="prepend">Key</template>
          </el-input>
        </el-row>
        <el-row>
          <el-input
            type="textarea"
            placeholder="Output"
            v-model="commandOutput"
            :rows="10"
            :disabled="true"></el-input>
        </el-row>
      </div>
    </div>
  </div>
</template>

<script>
import { Loading } from 'element-ui';
import UUID from './sugar-uuid'
for (let i in UUID) {
  UUID[i] = UUID[i].toLowerCase()
}
export default {
  name: 'App',
  components: {

  },
  data () {
    return {
      supportBluetooth: false,
      isConnected: false,
      serverId: '',
      ssid: '',
      password: '',
      key: 'pisugar',
      commandOutput: '',
      characteristics: [],
      infoList: [],
      commandList: [],
      loading: null
    }
  },
  mounted () {
    this.supportBluetooth = window.navigator.bluetooth
  },
  methods: {
    connectDevice () {
      const that = this
      navigator.bluetooth.requestDevice({
        filters: [{
          services: [UUID.SERVICE_ID]
        }]
      })
        .then(device => {
          that.loading = Loading.service({ fullscreen: true })
          return device.gatt.connect()
        })
        .then(server => {
          that.serverId = server.device.id
          return server.getPrimaryService(UUID.SERVICE_ID)
        })
        .then(service => service.getCharacteristics())
        .then(characteristics => {
          that.characteristics = characteristics
          that.isConnected = true
          that.loading.close()
          // console.log(that.characteristics)
        })
        .catch(console.log)
    },
    ab2str (buf) {
      return String.fromCharCode.apply(null, new Uint8Array(buf));
    },
    getCharacteristic (uuid) {
      return this.characteristics.find(i => i.uuid === uuid)
    },
    subscribeCharacteristic (uuid) {
      this.getCharacteristic(uuid).addEventListener('characteristicvaluechanged', event => {
        if (event.target.uuid === UUID.NOTIFY_MESSAGE) {
          let msg = this.ab2str(event.target.value.buffer)
          console.log(msg)
        } else if (event.target.uuid === UUID.CUSTOM_COMMAND_NOTIFY) {
          let msg = this.ab2str(event.target.value.buffer)
          console.log(msg)
        } else {
          this.infoList.find(i => i.uuid === uuid).value = this.ab2str(event.target.value.buffer)
        }
      })
      this.getCharacteristic(uuid).startNotifications()
    },
    readInfoCharacteristic (uuid) {
      this.getCharacteristic(uuid).readValue()
        .then(i => i.buffer)
        .then(this.ab2str)
        .then(this.updateFunc(uuid))
    },
    readCommandLabel (uuid) {
      this.getCharacteristic(uuid).readValue()
        .then(i => i.buffer)
        .then(this.ab2str)
        .then((label) => {
          this.commandList.find(i => i.uuid === uuid).label = label
        })
    },
    updateFunc (uuid) {
      let char = this.infoList.find(i => i.uuid === uuid)
      let char_label = this.infoList.find(i => i.uuid_label === uuid)
      return value => {
        if (char) {
          char.value = value
        } else {
          char_label.label = value
        }
      }
    },
    inputWifi () {
      let key = this.key.trim().replace(/\|/g, "*")
      let ssid = this.ssid.trim().replace(/\|/g, "*")
      let password = this.password.trim().replace(/\|/g, "*")
      let errMsg
      if (ssid === '') {
        errMsg = 'SSID不能为空'
      }
      if (password === '') {
        errMsg = '密码不能为空'
      }
      if (errMsg) {
        console.log('error')
        return
      }
      let sendArray = this.str2abs(`${key}%&%${ssid}%&%${password}&#&`)
      this.sendSeparately(sendArray, UUID.INPUT_SEP)
    },
    async sendSeparately (array, uuid) {
      for (const i in array) {
        console.log(`sending wifi setting: ${array[i]}`)
        await this.getCharacteristic(uuid).writeValue(array[i])
        await this.wait(0.4)
      }
    },
    async wait (sec) {
      return new Promise((resolve => {
        setTimeout(() => {
          resolve(true)
        }, 1000 * sec)
      }))
    },
    str2abs(str) {
      let val = ''
      for (let i = 0; i < str.length; i++) {
        if (val === '') {
          val = str.charCodeAt(i).toString(16)
        } else {
          val += ',' + str.charCodeAt(i).toString(16)
        }
      }
      let valArray = val.split(',')
      let bufferArray = []
      while (valArray.length > 0) {
        let value = valArray.splice(0, 20).join(',')
        bufferArray.push(new Uint8Array(value.match(/[\da-f]{2}/gi).map(function (h) {
          return parseInt(h, 16)
        })).buffer)
      }
      return bufferArray
    }
  },
  watch: {
    isConnected (val, oldVal) {
      if (val === true && oldVal === false) {
        console.log('Connected!')
        this.infoList = []
        this.infoList.push({
          uuid: '',
          label: 'Device ID',
          value: this.serverId
        })
        this.infoList.push({
          uuid: UUID.DEVICE_MODEL,
          label: 'Model',
          value: ' '
        })
        this.infoList.push({
          uuid: UUID.WIFI_NAME,
          label: 'Wifi',
          value: ' '
        })
        this.infoList.push({
          uuid: UUID.IP_ADDRESS,
          label: 'IP Address',
          value: ' '
        })
        this.subscribeCharacteristic(UUID.WIFI_NAME)
        this.subscribeCharacteristic(UUID.IP_ADDRESS)
        this.readInfoCharacteristic(UUID.DEVICE_MODEL)
        this.subscribeCharacteristic(UUID.NOTIFY_MESSAGE)
        this.subscribeCharacteristic(UUID.CUSTOM_COMMAND_NOTIFY)
        this.characteristics.filter(i => i.uuid.indexOf(UUID.CUSTOM_INFO_LABEL) >= 0).map(i => {
          this.infoList.push({
            uuid: i.uuid.replace(UUID.CUSTOM_INFO_LABEL, UUID.CUSTOM_INFO),
            uuid_label: i.uuid,
            label: '',
            value: ''
          })
          this.readInfoCharacteristic(i.uuid)
          this.subscribeCharacteristic(i.uuid.replace(UUID.CUSTOM_INFO_LABEL, UUID.CUSTOM_INFO))
        })
        this.characteristics.filter(i => i.uuid.indexOf(UUID.CUSTOM_COMMAND_LABEL) >= 0).map(i => {
          this.commandList.push({
            uuid: i.uuid,
            label: '...'
          })
          this.readCommandLabel(i.uuid)
        })
      }
    }
  }
}
</script>

<style lang="less">
  *{
    box-sizing: border-box;
  }
  html{
    width: 100%;
    height: 100%;
    background-color: orange;
    padding: 0;
    margin: 0;
  }
  body{
    padding: 10px 16px;
    margin: 0;
    background: linear-gradient(#ffe025, orange);
  }
  .logo{
    width: 160px;
  }
  #app {
    font-family: 'Avenir', Helvetica, Arial, sans-serif;
    -webkit-font-smoothing: antialiased;
    -moz-osx-font-smoothing: grayscale;
    text-align: center;
    color: #2c3e50;
    margin-top: 30px;
  }
  .panel{
    position: relative;
    width: 100%;
    min-height: 50px;
    background-color: #fff;
    margin-top: 20px;
    border-radius: 6px;
    box-shadow: 0 0 5px 2px rgba(230, 157, 42, 0.08);
    padding: 15px 20px;
    text-align: left;
    &:after{
      content: ' ';
      display: block;
      clear: both;
    }
  }
  @media screen and (min-width: 900px) {
    .panel-container {
      display: flex;
      justify-content: space-between;
    }
    .panel{
      margin-right: 10px;
    }
  }
  .connect-panel{
    p{
      color: #999;
    }
  }
  .el-row{
    margin-bottom: 15px;
  }
  .command-panel{
    .button-wrap{
      float: left;
      margin: 0 10px 10px 0;
    }
  }
  .panel-title{
    width: 100%;
    height: 30px;
    text-align: left;
    font-size: 16px;
    margin-bottom: 12px;
    font-weight: bold;
  }
  .info-group{
    position: relative;
    float: left;
    display: block;
    text-align: left;
    width: 48%;
    margin-bottom: 20px;
    margin-right: 1%;
    overflow: hidden;
    min-height: 45px;
    .label{
      font-size: 14px;
      font-weight: bold;
      color: #758699;
    }
    .value{
      width: 96%;
      font-size: 14px;
      overflow:hidden;
      text-overflow:ellipsis;
    }
  }
  .code{
    display: block;
    width: 100%;
    height: 100px;
    background-color: #333333;
    color: #e4edff;
    padding: 6px 10px;
  }
</style>
