import TitleBar from './TitleBar';
import CliWebView from './CliWebView';
import TerminalPage from './terminal/TerminalPage';
import { useTerminal } from '../context/TerminalContext';

// TODO: re-enable terminal view when terminal feature is ready
const SHOW_TERMINAL = false;

export default function MainView() {
  const { activeView } = useTerminal();

  return (
    <>
      <TitleBar />
      <div className="flex-1 flex flex-col overflow-hidden relative">
        <div
          className={!SHOW_TERMINAL || activeView === 'webui' ? 'flex-1 flex flex-col' : 'hidden'}
        >
          <CliWebView />
        </div>
        {SHOW_TERMINAL && (
          <div className={activeView === 'terminal' ? 'flex-1 flex flex-col' : 'hidden'}>
            <TerminalPage />
          </div>
        )}
      </div>
    </>
  );
}
