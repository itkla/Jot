// import { invoke } from '@tauri-apps/api/core';

let currentFilePath = null;
let isModified = false;
let editor;
let titleUpdatePending = false;

// simple debounce utility
function debounce(fn, delay) {
    let timer;
    return (...args) => {
        clearTimeout(timer);
        timer = setTimeout(() => fn(...args), delay);
    };
}

const invoke = window.__TAURI__.core.invoke;

// Debounced title update to avoid excessive calls
function debouncedUpdateTitle() {
    if (titleUpdatePending) return;
    
    titleUpdatePending = true;
    requestAnimationFrame(() => {
        const fileName = currentFilePath ? currentFilePath.split('/').pop() : null;
        invoke('update_title', { 
            filename: fileName, 
            isModified: isModified 
        }).catch(console.error).finally(() => {
            titleUpdatePending = false;
        });
    });
}

function init() {
    editor = document.getElementById('editor');
    
    if (!editor) {
        console.error('Editor element not found!');
        return;
    }
    if (!window.__TAURI__?.core?.invoke) {
        console.error('Tauri core invoke API not found');
        return;
    }
    
    // optimized event listeners
    editor.addEventListener('input', debounce(onEditorChange, 100), { passive: true });
    document.addEventListener('keydown', handleKeyboardShortcuts);
    
    // Initialize
    debouncedUpdateTitle();
    editor.focus();
}

// editor change handler
function onEditorChange() {
    if (!isModified) {
        isModified = true;
        debouncedUpdateTitle();
    }
}

// file operations
async function newFile() {
    try {
        await invoke('new_file');
    } catch (error) {
        console.error('Error creating new window:', error);
    }
}

async function openFile() {
    try {
        const result = await invoke('open_file_with_confirmation', {
            hasUnsavedChanges: isModified
        });
        
        if (result) {
            const [content, filePath] = result;
            editor.value = content;
            currentFilePath = filePath;
            isModified = false;
            editor.focus();
        }
    } catch (error) {
        console.error('Error opening file:', error);
    }
}

async function saveFile() {
    try {
        const result = await invoke('save_file', {
            filePath: currentFilePath,
            content: editor.value
        });
        if (result) {
            currentFilePath = result;
            isModified = false;
        }
    } catch (error) {
        console.error('Error saving file:', error);
    }
}

async function saveAsFile() {
    try {
        const result = await invoke('save_as_file', {
            content: editor.value
        });
        if (result) {
            currentFilePath = result;
            isModified = false;
        }
    } catch (error) {
        console.error('Error saving file:', error);
    }
}

async function clearDocument() {
    try {
        const shouldClear = await invoke('clear_document_with_confirmation', {
            hasUnsavedChanges: isModified
        });
        
        if (shouldClear) {
            editor.value = '';
            currentFilePath = null;
            isModified = false;
            editor.focus();
        }
    } catch (error) {
        console.error('Error clearing document:', error);
    }
}

// keyboard shortcuts
function handleKeyboardShortcuts(e) {
    const modifier = e.ctrlKey || e.metaKey;
    
    if (modifier && e.key === 'n') {
        e.preventDefault();
        if (e.shiftKey) {
            clearDocument();
        } else {
            newFile();
        }
    } else if (modifier && e.key === 'o') {
        e.preventDefault();
        openFile();
    } else if (modifier && e.key === 's') {
        e.preventDefault();
        if (e.shiftKey) {
            saveAsFile();
        } else {
            saveFile();
        }
    }
}

// Initialize when DOM is loaded
document.addEventListener('DOMContentLoaded', init);

