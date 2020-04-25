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
function fillBasicDemo() {
    let searchTermsArea = document.getElementById("searchTermsArea");
    let searchStagesArea = document.getElementById("searchStagesArea");
    let flavortextArea = document.getElementById("flavortextArea");

    searchTermsArea.textContent = 'manning,bradshaw,brady';
    searchStagesArea.textContent = 'WikiArticleStem';
    flavortextArea.textContent = '';
}

function fillHomophoneDemo() {
    let searchTermsArea = document.getElementById("searchTermsArea");
    let searchStagesArea = document.getElementById("searchStagesArea");
    let flavortextArea = document.getElementById("flavortextArea");

    searchTermsArea.textContent = 'symbol,coral,cord,loot';
    searchStagesArea.textContent = 'Homophone,WikiArticleStem';
    flavortextArea.textContent = '';
}

function fillOneStageDemo() {
    let searchTermsArea = document.getElementById("searchTermsArea");
    let searchStagesArea = document.getElementById("searchStagesArea");
    let flavortextArea = document.getElementById("flavortextArea");

    searchTermsArea.textContent = 'cats-eye,red devil,tom bowler,agate';
    searchStagesArea.textContent = 'WikiAllStem';
    flavortextArea.textContent = '';
}

function fillTwoStageDemo() {
    let searchTermsArea = document.getElementById("searchTermsArea");
    let searchStagesArea = document.getElementById("searchStagesArea");
    let flavortextArea = document.getElementById("flavortextArea");

    searchTermsArea.textContent = 'head,money,skip,pronounce';
    searchStagesArea.textContent = 'Synonym,WikiArticleStem';
    flavortextArea.textContent = 'moon';
    clearAndFillDisplayArea('This is a longer demo inspired by https://pennypark.fun/puzzle/spaceopolis/solution');
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

function truncateSearchMatch(searchTerm, searchMatch) {
    let index = searchMatch.indexOf(searchTerm);
    if (index == -1) {
        return searchMatch.substring(0, 30);
    } else {
        let start = index - 15;
        let end = index + searchTerm.length + 15;
        var prefix = '';
        var suffix = '';
        if (start > 0) {
            prefix = '...';
        }
        if (end < searchMatch.length) {
            suffix = '...'
        }
        return prefix + searchMatch.substring(Math.max(0, index - 15), Math.min(index + searchTerm.length + 15, searchMatch.length)) + suffix;
    }
}

function formatSingleChain(chain) {
    let EXPLANATIONS = {
        'Homophone': '[{2}] is a homophone of [{0}]',
        'Synonym': '[{2}] is a synonym of [{0}]',
        'WikiArticleStem': 'article [{2}] contains article [{1}] which stem-matched [{0}]',
        'WikiArticleExact': 'article [{2}] contains [{1}]',
        'WikiAllStem': 'article [{2}] has table/list item or article [{2}] which stem-matched [{0}]',
    };
    var explanations = [];
    var finalResult = "";
    for (i = 0; i < chain.length; i += 4) {
        let stage = chain[i];
        let searchTerm = chain[i+1];
        let searchMatch = truncateSearchMatch(searchTerm, chain[i+2]);
        let searchResult = chain[i+3];
        finalResult = searchResult;
        let template = EXPLANATIONS[stage];
        let explanationString = template.replace('{0}', searchTerm).replace('{1}', searchMatch).replace('{2}', searchResult);
        explanations.push(explanationString);
    }
    return {
        'finalAssociation': finalResult,
        'explanations': explanations.reverse()
    }
}

function formatResponse(responseArray) {
    // General format is a list of dictionaries
    // search_term => list of steps
    // where each step is [StageName, search_term, search_match, result]
    var pieces = [];
    for (var i = 0; i < responseArray.length; i++) {
        let result = responseArray[i];
        var lines = []
        var association = "";
        for (var key in result) {
            let processedChain = formatSingleChain(result[key]);
            // All associatios will be the same
            if (processedChain['finalAssociation']) {
                association = processedChain['finalAssociation'];
                lines.push("- " + processedChain['explanations'].join(' <= '));
            } else {
                lines.push("- nothing found for term: [" + key + "]");
            }
        }
        var finalString = "[" + association + "]\n" + lines.join("\n");
        pieces.push(finalString);
    }
    clearAndFillDisplayArea(pieces.join("\n\n"));
}

function handleResponse(responseText) {
    var responseObject = JSON.parse(responseText);
    let displayArea = document.getElementById("displayArea");
    if (responseObject['error']) {
        clearAndFillDisplayArea(responseObject['error']);
    } else {
        formatResponse(responseObject)
    }
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
            handleResponse(this.responseText);
            hideWaiting();
        }
    }
    xhr.send(JSON.stringify(payload));
    showWaiting();
    clearAndFillDisplayArea('');
}

// === UI Functions ===

function showWaiting() {
  document.getElementById('waitingAlert').setAttribute('style', 'display:block;');
}

function hideWaiting() {
  document.getElementById('waitingAlert').setAttribute('style', 'display:none;');
}

function clearAndFillDisplayArea(text) {
  document.getElementById('displayArea').textContent = text;
}
