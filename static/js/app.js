// MIT License
//
// Copyright (c) 2020 Michael Xu
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

// === Demo Functions ===

function fillSynonymDemo() {
    let searchTermsArea = document.getElementById("searchTermsArea");
    let searchStagesArea = document.getElementById("searchStagesArea");
    let flavortextArea = document.getElementById("flavortextArea");

    searchTermsArea.textContent = 'symbol,coral,cord,loot';
    searchStagesArea.textContent = 'Homophone,WikiArticleStem';
    flavortextArea.textContent = '';
}

function addStage(stageName) {
    let searchStagesArea = document.getElementById("searchStagesArea");
    console.log(searchStagesArea.textContent);
    var stages = searchStagesArea.textContent.split(',');
    console.log(stages);
    if (stages.length == 1 && stages[0] === '') {
        stages = [stageName];
    } else {
        stages.push(stageName);
    }
    searchStagesArea.textContent = stages.join(',');
}

function removeStage() {
    let searchStagesArea = document.getElementById("searchStagesArea");
    var stages = searchStagesArea.textContent.split(',');
    if (stages.length > 0) {
        stages.pop();
    }
    searchStagesArea.textContent = stages.join(',');
}

function clearFlavortext() {
    let flavortextArea = document.getElementById("flavortextArea");
    flavortextArea.textContent = '';
}

function handleResponse(response) {
}

function executeQuery() {
    let searchTermsArea = document.getElementById("searchTermsArea");
    let searchStagesArea = document.getElementById("searchStagesArea");
    let flavortextArea = document.getElementById("flavortextArea");

    let payload = {
        'stages': searchStagesArea.textContent.split(','),
        'terms': searchTermsArea.textContent.split(','),
        'flavortext': flavortextArea.textContent
    }

    var xhr = new XMLHttpRequest();
    xhr.open('POST', '/query', true);
    xhr.setRequestHeader("Content-Type", "application/json");
    
    xhr.onreadystatechange = function() { // Call a function when the state changes.
        if (this.readyState === XMLHttpRequest.DONE && this.status === 200) {
            // Request finished. Do processing here.
            console.log(this);
            handleResponse(this);
        }
    }
    xhr.send(JSON.stringify(payload));
}

// === UI Functions ===

function showWaiting() {
  document.getElementById('waitingAlert').setAttribute('style', 'display:block;');
}

function hideWaiting() {
  document.getElementById('waitingAlert').setAttribute('style', 'display:none;');
}

function clearAndFillDebugArea(text) {
  document.getElementById('debugArea').textContent = text;
}

function formatOpensslInstructions(sigEncoded, payloadEncoded, exportedPubKeyEncoded) {
  var instructionBlock = 'echo "' + sigEncoded + '" | base64 --decode > sig.bin;\n';
  instructionBlock += 'echo "' + payloadEncoded + '" | base64 --decode > payload.bin;\n';
  instructionBlock += 'echo "' + exportedPubKeyEncoded + '" | base64 --decode > ec.pub;\n';
  instructionBlock += 'openssl dgst -sha256 -verify ec.pub -signature sig.bin payload.bin;'
  return instructionBlock;
}
