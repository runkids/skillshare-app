import TitleBar from './TitleBar';
import CliWebView from './CliWebView';
import TerminalPage from './terminal/TerminalPage';
import { useTerminal } from '../context/TerminalContext';

export default function MainView() {
  const { activeView } = useTerminal();

  return (
    <>
      <TitleBar />
      <div className="flex-1 flex flex-col overflow-hidden relative">
        <div className={activeView === 'webui' ? 'flex-1 flex flex-col' : 'hidden'}>
          <CliWebView />
        </div>
        <div className={activeView === 'terminal' ? 'flex-1 flex flex-col' : 'hidden'}>
          <TerminalPage />
        </div>
      </div>
    </>
  );
}
